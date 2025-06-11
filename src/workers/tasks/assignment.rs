use bevy::prelude::*;
use crate::{
    grid::Position,
    materials::Inventory,
    workers::{manhattan_distance_coords, AssignedSequence, Worker, WorkerState}
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