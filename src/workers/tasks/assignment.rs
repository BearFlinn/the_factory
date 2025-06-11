use bevy::prelude::*;
use crate::{
    grid::Position, materials::{Inventory, InventoryType, InventoryTypes}, structures::Building, workers::{manhattan_distance_coords, AssignedSequence, Worker, WorkerPath, WorkerState}
};
use super::components::*;

pub fn assign_available_sequences_to_workers(
    mut sequences: Query<(Entity, &mut AssignedWorker, &TaskSequence, &Priority)>,
    mut workers: Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
    tasks: Query<&Position, With<Task>>,
    _time: Res<Time>,
) {
    cleanup_orphaned_assignments(&mut sequences, &mut workers);
    
    let mut unassigned_sequences: Vec<_> = sequences
        .iter_mut()
        .filter(|(_, assigned_worker, sequence, _)| {
            assigned_worker.0.is_none() && !sequence.is_complete()
        })
        .collect();
    
    unassigned_sequences.sort_by_key(|(_, _, _, priority)| {
        match priority {
            Priority::Critical => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
        }
    });
    
    for (sequence_entity, mut assigned_worker, sequence, _) in unassigned_sequences {
        let current_task_entity = match sequence.current_task() {
            Some(task) => task,
            None => continue,
        };
        
        let task_position = match tasks.get(current_task_entity) {
            Ok(pos) => pos,
            Err(_) => continue,
        };
        
        let task_pos = (task_position.x, task_position.y);
        
        if let Some((worker_entity, _)) = find_available_worker(task_pos, &workers) {
            assigned_worker.0 = Some(worker_entity);
            
            if let Ok((_, _, _, mut worker_assigned_sequence, _)) = workers.get_mut(worker_entity) {
                worker_assigned_sequence.0 = Some(sequence_entity);
            } else {
                assigned_worker.0 = None;
            }
        }
    }
}

pub fn derive_worker_state_from_sequences(
    mut workers: Query<(&mut AssignedSequence, &mut WorkerState), With<Worker>>,
    mut sequences: Query<&mut TaskSequence>,
    tasks: Query<Entity, With<Task>>,
) {
    for (mut assigned_sequence, mut worker_state) in workers.iter_mut() {
        let new_state = match assigned_sequence.0 {
            None => WorkerState::Idle,
            Some(sequence_entity) => {
                match sequences.get_mut(sequence_entity) {
                    Ok(mut sequence) => {
                        sequence.validate_and_advance(&tasks);
                        
                        if sequence.is_complete_with_validation(&tasks) {
                            assigned_sequence.0 = None;
                            WorkerState::Idle
                        } else {
                            WorkerState::Working
                        }
                    }
                    Err(_) => {
                        assigned_sequence.0 = None;
                        WorkerState::Idle
                    }
                }
            }
        };
        
        if *worker_state != new_state {
            *worker_state = new_state;
        }
    }
}

pub fn clear_all_tasks(
    mut commands: Commands,
    query: Query<Entity, With<Task>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        for entity in query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn clear_completed_tasks(
    mut commands: Commands, 
    query: Query<(Entity, &TaskStatus), With<Task>>,
) {
    for (entity, status) in query.iter() {
        if *status == TaskStatus::Completed {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn cleanup_orphaned_assignments(
    sequences: &mut Query<(Entity, &mut AssignedWorker, &TaskSequence, &Priority)>,
    workers: &mut Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
) {
    let mut orphaned_sequences = Vec::new();
    
    for (sequence_entity, assigned_worker, _, _) in sequences.iter() {
        if let Some(worker_entity) = assigned_worker.0 {
            if let Ok((_, _, _, worker_assigned_sequence, _)) = workers.get(worker_entity) {
                if worker_assigned_sequence.0 != Some(sequence_entity) {
                    orphaned_sequences.push(sequence_entity);
                }
            } else {
                orphaned_sequences.push(sequence_entity);
            }
        }
    }
    
    for sequence_entity in orphaned_sequences {
        if let Ok((_, mut assigned_worker, _, _)) = sequences.get_mut(sequence_entity) {
            assigned_worker.0 = None;
        }
    }
}

fn find_available_worker(
    position: (i32, i32),
    workers: &Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
) -> Option<(Entity, Position)> {
    let mut best_worker = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, pos, worker_state, assigned_sequence, inventory) in workers.iter() {
        let is_available = assigned_sequence.0.is_none() && 
                                *worker_state == WorkerState::Idle && 
                                inventory.is_empty();
        
        if is_available {
            let distance = manhattan_distance_coords(position, (pos.x, pos.y));
            if distance < closest_distance {
                closest_distance = distance;
                best_worker = Some((entity, *pos));
            }
        }
    }
    
    best_worker
}

pub fn handle_worker_interrupts(
    mut commands: Commands,
    mut interrupt_events: EventReader<WorkerInterruptEvent>,
    mut workers: Query<(&mut AssignedSequence, &mut WorkerState, &mut WorkerPath), With<Worker>>,
    mut sequences: Query<&mut AssignedWorker>,
) {
    for event in interrupt_events.read() {
        let Ok((mut worker_assigned_sequence, mut worker_state, mut worker_path)) = workers.get_mut(event.worker) else {
            println!("WorkerInterrupt: Worker entity {:?} not found", event.worker);
            continue;
        };

        // Clean up old assignment
        if let Some(old_sequence_entity) = worker_assigned_sequence.0 {
            if let Ok(mut old_assigned_worker) = sequences.get_mut(old_sequence_entity) {
                old_assigned_worker.0 = None;
            }
        }

        // Clear worker's pathfinding state for clean transition
        worker_path.waypoints.clear();
        worker_path.current_target = None;

        // Apply the interrupt
        match &event.interrupt_type {
            InterruptType::ReplaceSequence(new_sequence_entity) => {
                // Verify the new sequence exists and assign it
                if let Ok(mut new_assigned_worker) = sequences.get_mut(*new_sequence_entity) {
                    worker_assigned_sequence.0 = Some(*new_sequence_entity);
                    new_assigned_worker.0 = Some(event.worker);
                    *worker_state = WorkerState::Working;
                    
                    println!("WorkerInterrupt: Worker {:?} assigned to sequence {:?}", 
                             event.worker, new_sequence_entity);
                } else {
                    // Sequence doesn't exist, clear assignment
                    worker_assigned_sequence.0 = None;
                    *worker_state = WorkerState::Idle;
                    
                    println!("WorkerInterrupt: New sequence {:?} not found, worker {:?} set to idle", 
                             new_sequence_entity, event.worker);
                }
            }
            
            InterruptType::ReplaceTasks(new_tasks, priority) => {
                if !new_tasks.is_empty() {
                    // Create new sequence from tasks
                    let new_sequence_entity = commands.spawn(
                        TaskSequenceBundle::new(new_tasks.clone(), priority.clone())
                    ).id();
                    
                    // Assign to worker
                    worker_assigned_sequence.0 = Some(new_sequence_entity);
                    *worker_state = WorkerState::Working;
                    
                    // Update sequence's assigned worker (need to do this in a deferred way)
                    commands.entity(new_sequence_entity).insert(AssignedWorker(Some(event.worker)));
                    
                    // Add SequenceMember to tasks
                    for &task_entity in new_tasks {
                        commands.entity(task_entity).insert(SequenceMember(new_sequence_entity));
                    }
                    
                    println!("WorkerInterrupt: Worker {:?} assigned to new sequence {:?} with {} tasks", 
                             event.worker, new_sequence_entity, new_tasks.len());
                } else {
                    // Empty task list, clear assignment
                    worker_assigned_sequence.0 = None;
                    *worker_state = WorkerState::Idle;
                    
                    println!("WorkerInterrupt: Empty task list, worker {:?} set to idle", event.worker);
                }
            }
            
            InterruptType::ClearAssignment => {
                worker_assigned_sequence.0 = None;
                *worker_state = WorkerState::Idle;
                
                println!("WorkerInterrupt: Worker {:?} assignment cleared", event.worker);
            }
        }
    }
}

/// Debug system: Clear all worker assignments when spacebar is pressed
pub fn debug_clear_all_workers(
    keys: Res<ButtonInput<KeyCode>>,
    workers: Query<Entity, With<Worker>>,
    mut interrupt_events: EventWriter<WorkerInterruptEvent>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let worker_count = workers.iter().count();
        
        for worker_entity in workers.iter() {
            interrupt_events.send(WorkerInterruptEvent {
                worker: worker_entity,
                interrupt_type: InterruptType::ClearAssignment,
            });
        }
        
        if worker_count > 0 {
            println!("Debug: Cleared assignments for {} workers", worker_count);
        }
    }
}

/// Temporary system: Create dropoff tasks for idle workers carrying items
/// This will be replaced by the error handling system later
pub fn emergency_dropoff_idle_workers(
    mut commands: Commands,
    workers: Query<(Entity, &Position, &WorkerState, &AssignedSequence, &Inventory), With<Worker>>,
    storage_buildings: Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    mut interrupt_events: EventWriter<WorkerInterruptEvent>,
) {
    for (worker_entity, worker_pos, worker_state, assigned_sequence, worker_inventory) in workers.iter() {
        // Only process idle workers with items and no current assignment
        if *worker_state != WorkerState::Idle || 
           assigned_sequence.0.is_some() || 
           worker_inventory.is_empty() {
            continue;
        }
        
        // Find nearest storage with available space
        let worker_grid_pos = (worker_pos.x, worker_pos.y);
        let nearest_storage = find_nearest_available_storage(
            worker_grid_pos, 
            &storage_buildings
        );
        
        if let Some((storage_entity, storage_pos)) = nearest_storage {
            // Get all items from worker inventory
            let worker_items = worker_inventory.get_all_items();
            
            if !worker_items.is_empty() {
                // Create pickup task (worker → temporary holding)
                let pickup_task = commands.spawn(TaskBundle::new(
                    worker_entity,
                    *worker_pos,
                    TaskAction::Pickup(Some(worker_items.clone())),
                    Priority::Medium,
                )).id();
                
                // Create dropoff task (temporary holding → storage)
                let dropoff_task = commands.spawn(TaskBundle::new(
                    storage_entity,
                    storage_pos,
                    TaskAction::Dropoff(Some(worker_items)),
                    Priority::Medium,
                )).id();
                
                // Send interrupt to assign the emergency sequence
                interrupt_events.send(WorkerInterruptEvent {
                    worker: worker_entity,
                    interrupt_type: InterruptType::ReplaceTasks(
                        vec![pickup_task, dropoff_task], 
                        Priority::Medium
                    ),
                });
                
                println!("Emergency: Created dropoff sequence for worker {:?} → storage {:?}", 
                         worker_entity, storage_entity);
            }
        }
    }
}

/// Find the nearest storage building with available inventory space
fn find_nearest_available_storage(
    worker_pos: (i32, i32),
    storage_buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
) -> Option<(Entity, Position)> {
    let mut nearest_storage = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, position, inventory, inventory_type) in storage_buildings.iter() {
        // Only consider storage buildings with available space
        if inventory_type.0 != InventoryTypes::Storage || inventory.is_full() {
            continue;
        }
        
        let storage_pos = (position.x, position.y);
        let distance = manhattan_distance_coords(worker_pos, storage_pos);
        
        if distance < closest_distance {
            closest_distance = distance;
            nearest_storage = Some((entity, *position));
        }
    }
    
    nearest_storage
}