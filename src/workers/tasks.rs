use std::collections::{HashMap, VecDeque};

use bevy::{prelude::*};

use crate::{
    grid::{Grid, Position},
    materials::{request_transfer_all_items, request_transfer_specific_items, Inventory, InventoryType, InventoryTypes, ItemName, ItemTransferRequestEvent, RecipeRegistry},
    structures::{Building, CrafterLogisticsRequest, RecipeCrafter},
    systems::NetworkConnectivity,
    workers::{calculate_path, manhattan_distance_coords, AssignedSequence, Worker, WorkerArrivedEvent, WorkerPath, WorkerState}
};

#[derive(Component)]
pub struct Task;

#[derive(Component)]
#[allow(dead_code)] // TODO: Implement priority
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Component, PartialEq)]
pub enum TaskStatus {
    Pending,
    Queued,
    InProgress,
    Completed,
}

#[derive(Component, Clone, Debug)]
pub enum TaskAction {
    Pickup(Option<HashMap<ItemName, u32>>),
    Dropoff(Option<HashMap<ItemName, u32>>),
}

#[derive(Component)]
pub struct TaskTarget(pub Entity);

#[derive(Component)]
pub struct SequenceMember(pub Entity);

#[derive(Component)]
pub struct AssignedWorker(pub Option<Entity>);

#[derive(Bundle)]
pub struct TaskBundle {
    task: Task,
    priority: Priority,
    position: Position,
    task_status: TaskStatus,
    task_target: TaskTarget,
    task_action: TaskAction,
    assigned_worker: AssignedWorker,
}

impl TaskBundle {
    pub fn new(task_target: Entity, position: Position, action: TaskAction, priority: Priority) -> Self {
        Self {
            task: Task,
            priority: priority,
            position: position,
            task_status: TaskStatus::Pending,
            task_target: TaskTarget(task_target),
            task_action: action,
            assigned_worker: AssignedWorker(None),
        }
    }
}

#[derive(Component)]
pub struct TaskSequence {
    pub tasks: VecDeque<Entity>,
    pub current_index: usize,
}

impl TaskSequence {
    pub fn new(tasks: Vec<Entity>) -> Self {
        Self {
            tasks: VecDeque::from(tasks),
            current_index: 0,
        }
    }
    
    pub fn current_task(&self) -> Option<Entity> {
        self.tasks.get(self.current_index).copied()
    }
    
    pub fn advance_to_next(&mut self) -> Option<Entity> {
        self.current_index += 1;
        self.current_task()
    }
    
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.tasks.len()
    }
    
    pub fn remaining_tasks(&self) -> usize {
        self.tasks.len().saturating_sub(self.current_index)
    }

    pub fn is_complete_with_validation(&self, task_query: &Query<Entity, With<Task>>) -> bool {
        // First check the basic completion condition
        if self.current_index >= self.tasks.len() {
            return true;
        }
        
        // Check if current task entity still exists
        if let Some(current_task) = self.current_task() {
            if task_query.get(current_task).is_err() {
                return true; // Current task was despawned, consider sequence complete
            }
        }
        
        false
    }
    
    // Add method to validate and clean invalid tasks
    pub fn validate_and_advance(&mut self, task_query: &Query<Entity, With<Task>>) -> bool {
        let mut advanced = false;
        
        // Skip invalid tasks until we find a valid one or reach the end
        while self.current_index < self.tasks.len() {
            if let Some(current_task) = self.current_task() {
                if task_query.get(current_task).is_ok() {
                    break; // Found valid task
                }
            }
            
            // Current task is invalid, advance
            self.current_index += 1;
            advanced = true;
        }
        
        advanced
    }
}

#[derive(Bundle)]
pub struct TaskSequenceBundle {
    pub sequence: TaskSequence,
    pub priority: Priority,
    pub assigned_worker: AssignedWorker,
}

impl TaskSequenceBundle {
    pub fn new(tasks: Vec<Entity>, priority: Priority) -> Self {
        Self {
            sequence: TaskSequence::new(tasks),
            priority,
            assigned_worker: AssignedWorker(None),
        }
    }
}

pub fn process_worker_sequences_defensive(
    mut workers: Query<(Entity, &Transform, &mut AssignedSequence, &mut WorkerPath, &mut WorkerState), With<Worker>>,
    mut sequences: Query<&mut TaskSequence>,
    mut tasks: Query<(&Position, &mut TaskStatus, &TaskTarget), With<Task>>,
    task_targets: Query<Entity>, // For target existence validation
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>, // ADD: Event writer for immediate arrivals
) {
    for (worker_entity, transform, mut assigned_sequence, mut worker_path, mut worker_state) in workers.iter_mut() {
        // Skip workers without assigned sequences
        let Some(sequence_entity) = assigned_sequence.0 else {
            continue;
        };
        
        // Defensive: Check if sequence still exists
        let Ok(mut sequence) = sequences.get_mut(sequence_entity) else {
            // Sequence was despawned - free worker immediately
            assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
            continue;
        };
        
        // Check sequence completion
        if sequence.is_complete() {
            assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
            continue;
        }
        
        // Validate current task chain with early termination on failure
        let current_task_entity = match sequence.current_task() {
            Some(task) => task,
            None => {
                // Sequence has no valid current task - complete it
                assigned_sequence.0 = None;
                *worker_state = WorkerState::Idle;
                continue;
            }
        };
        
        // Defensive: Validate task entity exists
        let (task_position, mut task_status, task_target) = match tasks.get_mut(current_task_entity) {
            Ok(task_data) => task_data,
            Err(_) => {
                // Task entity missing - log and advance sequence
                println!("Warning: Task entity {:?} missing from sequence {:?}, advancing to next task", 
                         current_task_entity, sequence_entity);
                
                if sequence.advance_to_next().is_none() {
                    // No more tasks, complete sequence
                    assigned_sequence.0 = None;
                    *worker_state = WorkerState::Idle;
                }
                continue;
            }
        };
        
        // Defensive: Validate task target exists
        if task_targets.get(task_target.0).is_err() {
            // Task target destroyed - log and advance sequence
            println!("Warning: Task target {:?} destroyed for task {:?}, advancing sequence {:?}", 
                     task_target.0, current_task_entity, sequence_entity);
            
            *task_status = TaskStatus::Completed; // Mark as completed to prevent retry
            if sequence.advance_to_next().is_none() {
                // No more tasks, complete sequence
                assigned_sequence.0 = None;
                *worker_state = WorkerState::Idle;
                print!("Sequence {:?} completed", sequence_entity);
            }
            continue;
        }
        
        // All validation passed - proceed with normal task processing
        
        // Update current task status to InProgress
        if *task_status == TaskStatus::Pending || *task_status == TaskStatus::Queued {
            *task_status = TaskStatus::InProgress;
        }
        
        // Skip pathfinding if already moving
        if worker_path.current_target.is_some() || !worker_path.waypoints.is_empty() {
            continue;
        }
        
        // Initiate pathfinding for current task
        if let Some(worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
            let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
            let target_pos = (task_position.x, task_position.y);

            if let Some(path) = calculate_path(worker_pos, target_pos, &network, &grid) {
                worker_path.waypoints = path;
                worker_path.current_target = worker_path.waypoints.pop_front();
                
                // FIX: Handle spatial coincidence - worker already at target
                if worker_path.current_target.is_none() && worker_path.waypoints.is_empty() {
                    // Worker is already at the target position, immediately trigger arrival
                    arrival_events.send(WorkerArrivedEvent {
                        worker: worker_entity,
                        position: target_pos,
                    });
                }
            } else {
                // No path available - advance sequence to try next task
                *task_status = TaskStatus::Completed;
                if sequence.advance_to_next().is_none() {
                    assigned_sequence.0 = None;
                    *worker_state = WorkerState::Idle;
                }
            }
        }
    }
}

pub fn handle_sequence_task_arrivals_defensive(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    mut workers: Query<(&mut AssignedSequence, &Inventory, &mut WorkerState), With<Worker>>,
    mut sequences: Query<(&mut TaskSequence, &mut AssignedWorker)>,
    mut tasks: Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
    task_targets: Query<Entity>, // For target existence validation
    inventories: Query<&Inventory>,
    mut transfer_requests: EventWriter<ItemTransferRequestEvent>,
) {
    for event in arrival_events.read() {
        // Defensive: Get worker's assigned sequence
        let Ok((mut worker_assigned_sequence, worker_inventory, mut worker_state)) = workers.get_mut(event.worker) else {
            println!("Warning: Arrival event for invalid worker {:?}", event.worker);
            continue;
        };
        
        let Some(sequence_entity) = worker_assigned_sequence.0 else {
            println!("Warning: Arrival event for worker {:?} with no assigned sequence", event.worker);
            continue;
        };
        
        // Defensive: Get the sequence and its current task
        let Ok((mut sequence, mut sequence_assigned_worker)) = sequences.get_mut(sequence_entity) else {
            println!("Warning: Worker {:?} references invalid sequence {:?}", event.worker, sequence_entity);
            
            // Free the worker from invalid sequence
            worker_assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
            continue;
        };
        
        let Some(current_task_entity) = sequence.current_task() else {
            println!("Warning: Sequence {:?} has no current task", sequence_entity);
            
            // Complete the sequence
            worker_assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
            sequence_assigned_worker.0 = None;
            continue;
        };
        
        // Defensive: Verify task exists and get its details
        let Ok((task_position, task_action, task_target, mut task_status)) = tasks.get_mut(current_task_entity) else {
            println!("Warning: Sequence {:?} references invalid task {:?}", sequence_entity, current_task_entity);
            
            // Advance sequence past invalid task
            if sequence.advance_to_next().is_none() {
                worker_assigned_sequence.0 = None;
                *worker_state = WorkerState::Idle;
                sequence_assigned_worker.0 = None;
            }
            continue;
        };
        
        // Defensive: Verify task target still exists
        if task_targets.get(task_target.0).is_err() {
            println!("Warning: Task {:?} target {:?} was destroyed", current_task_entity, task_target.0);
            
            // Mark task as completed and advance sequence
            *task_status = TaskStatus::Completed;
            if sequence.advance_to_next().is_none() {
                worker_assigned_sequence.0 = None;
                *worker_state = WorkerState::Idle;
                sequence_assigned_worker.0 = None;
            }
            continue;
        };
        
        // Verify worker arrived at the correct task location
        let task_pos = (task_position.x, task_position.y);
        if event.position != task_pos {
            println!("Warning: Worker {:?} arrived at {:?} but task {:?} is at {:?}", 
                     event.worker, event.position, current_task_entity, task_pos);
            continue;
        }
        
        // All validation passed - execute task action
        match task_action {
            TaskAction::Pickup(items) => {
                if let Some(items) = items {
                    request_transfer_specific_items(
                        task_target.0,
                        event.worker,
                        items.clone(),
                        &mut transfer_requests,
                    );
                } else {
                    request_transfer_all_items(
                        task_target.0,
                        event.worker,
                        &mut transfer_requests,
                        &inventories,
                    );
                }
            }
            TaskAction::Dropoff(items) => {
                if let Some(items) = items {
                    request_transfer_specific_items(
                        event.worker,
                        task_target.0,
                        items.clone(),
                        &mut transfer_requests,
                    );
                } else {
                    let all_items = worker_inventory.get_all_items();
                    if !all_items.is_empty() {
                        request_transfer_specific_items(
                            event.worker,
                            task_target.0,
                            all_items,
                            &mut transfer_requests,
                        );
                    }
                }
            }
        }
        
        // Mark current task as completed
        *task_status = TaskStatus::Completed;
        
        // Advance sequence to next task
        if sequence.advance_to_next().is_none() {
            // Sequence is complete - clean up assignments
            *worker_state = WorkerState::Idle;
            worker_assigned_sequence.0 = None;
            sequence_assigned_worker.0 = None;
        }
    }
}

pub fn assign_available_sequences_to_workers(
    mut sequences: Query<(Entity, &mut AssignedWorker, &TaskSequence, &Priority)>,
    mut workers: Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
    tasks: Query<&Position, With<Task>>,
    time: Res<Time>,
) {
    // Only log every second to avoid spam
    let should_log = time.elapsed_secs() as i32 % 2 == 0;
    
    // STEP 1: Clean up orphaned assignments before new assignments
    cleanup_orphaned_assignments(&mut sequences, &mut workers);
    
    // STEP 2: Get truly unassigned sequences, sorted by priority
    let mut unassigned_sequences: Vec<_> = sequences
        .iter_mut()
        .filter(|(_, assigned_worker, sequence, _)| {
            assigned_worker.0.is_none() && !sequence.is_complete()
        })
        .collect();
    
    if should_log && !unassigned_sequences.is_empty() {
        println!("Found {} unassigned sequences", unassigned_sequences.len());
    }
    
    // Count truly available workers (no assigned sequence AND idle state)
    let available_workers = workers.iter()
        .filter(|(_, _, worker_state, assigned_sequence, inventory)| {
            assigned_sequence.0.is_none() && 
            (**worker_state == WorkerState::Idle && inventory.is_empty())
        })
        .count();
    
    if should_log && !unassigned_sequences.is_empty() {
        println!("Found {} truly available workers", available_workers);
    }
    
    // Sort by priority (Critical first, then High, Medium, Low)
    unassigned_sequences.sort_by_key(|(_, _, _, priority)| {
        match priority {
            Priority::Critical => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
        }
    });
    
    for (sequence_entity, mut assigned_worker, sequence, _) in unassigned_sequences {
        // Get position of current task in sequence for worker assignment calculation
        let current_task_entity = match sequence.current_task() {
            Some(task) => task,
            None => {
                if should_log {
                    println!("Sequence {:?} has no current task", sequence_entity);
                }
                continue;
            }
        };
        
        let task_position = match tasks.get(current_task_entity) {
            Ok(pos) => pos,
            Err(_) => {
                if should_log {
                    println!("Failed to get position for task {:?} in sequence {:?}", 
                             current_task_entity, sequence_entity);
                }
                continue;
            }
        };
        
        let task_pos = (task_position.x, task_position.y);
        
        // Find truly available worker (stricter criteria)
        let worker_result = find_truly_available_worker(task_pos, &workers);
        
        if let Some((worker_entity, _)) = worker_result {
            // ATOMIC BIDIRECTIONAL ASSIGNMENT
            assigned_worker.0 = Some(worker_entity);
            
            if let Ok((_, _, _, mut worker_assigned_sequence, _)) = workers.get_mut(worker_entity) {
                worker_assigned_sequence.0 = Some(sequence_entity);
                
                if should_log {
                    println!("Assigned sequence {:?} to worker {:?}", sequence_entity, worker_entity);
                }
            } else {
                // Rollback if worker assignment fails
                assigned_worker.0 = None;
                if should_log {
                    println!("Failed to update worker {:?} assignment, rolling back", worker_entity);
                }
            }
        } else if should_log {
            println!("No available worker found for sequence {:?} at position {:?}", 
                     sequence_entity, task_pos);
        }
    }
}

// NEW: Clean up orphaned assignments to maintain referential integrity
fn cleanup_orphaned_assignments(
    sequences: &mut Query<(Entity, &mut AssignedWorker, &TaskSequence, &Priority)>,
    workers: &mut Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
) {
    // Find sequences that think they have a worker, but the worker doesn't reference them back
    let mut orphaned_sequences = Vec::new();
    
    for (sequence_entity, assigned_worker, _, _) in sequences.iter() {
        if let Some(worker_entity) = assigned_worker.0 {
            // Check if worker actually references this sequence back
            if let Ok((_, _, _, worker_assigned_sequence, _)) = workers.get(worker_entity) {
                if worker_assigned_sequence.0 != Some(sequence_entity) {
                    orphaned_sequences.push(sequence_entity);
                }
            } else {
                // Worker entity doesn't exist - definitely orphaned
                orphaned_sequences.push(sequence_entity);
            }
        }
    }
    
    // Clean up orphaned sequences
    for sequence_entity in orphaned_sequences {
        if let Ok((_, mut assigned_worker, _, _)) = sequences.get_mut(sequence_entity) {
            println!("Cleaning up orphaned sequence assignment: {:?}", sequence_entity);
            assigned_worker.0 = None;
        }
    }
}

// NEW: Stricter worker availability check
fn find_truly_available_worker(
    position: (i32, i32),
    workers: &Query<(Entity, &Position, &WorkerState, &mut AssignedSequence, &Inventory), With<Worker>>,
) -> Option<(Entity, Position)> {
    let mut best_worker = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, pos, worker_state, assigned_sequence, inventory) in workers.iter() {
        // Worker is truly available ONLY if:
        // 1. No assigned sequence AND
        // 2. Idle state AND  
        // 3. Empty inventory
        let is_truly_available = assigned_sequence.0.is_none() && 
                                *worker_state == WorkerState::Idle && 
                                inventory.is_empty();
        
        if is_truly_available {
            let distance = manhattan_distance_coords(position, (pos.x, pos.y));
            if distance < closest_distance {
                closest_distance = distance;
                best_worker = Some((entity, *pos));
            }
        }
    }
    
    best_worker
}

pub fn create_logistics_tasks(
    mut commands: Commands,
    mut events: EventReader<CrafterLogisticsRequest>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    recipes: Res<RecipeRegistry>,
) {
    for event in events.read() {
        match (&event.needs, &event.has) {
            (Some(needed_items), None) => {
                // NEW: Create a single consolidated sequence per logistics request
                // instead of separate sequences per item type
                
                let supply_plan = calculate_supply_plan(
                    (event.position.x, event.position.y),
                    needed_items,
                    &buildings,
                );
                
                if !supply_plan.is_empty() {
                    let mut all_tasks = Vec::new();
                    
                    // Create pickup/dropoff pairs for each supply source
                    for (building_entity, building_pos, items_to_pickup) in supply_plan {
                        let pickup_task = commands.spawn((
                            TaskBundle::new(
                                building_entity,
                                building_pos,
                                TaskAction::Pickup(Some(items_to_pickup.clone())),
                                Priority::Medium,
                            ),
                        )).id();
                        
                        let dropoff_task = commands.spawn((
                            TaskBundle::new(
                                event.crafter,
                                event.position,
                                TaskAction::Dropoff(Some(items_to_pickup)),
                                Priority::Medium,
                            ),
                        )).id();
                        
                        all_tasks.push(pickup_task);
                        all_tasks.push(dropoff_task);
                    }
                    
                    // Create single sequence containing all pickup/dropoff pairs
                    if !all_tasks.is_empty() {
                        let sequence_entity = commands.spawn(
                            TaskSequenceBundle::new(all_tasks.clone(), Priority::Medium)
                        ).id();
                        
                        // Link all tasks to the sequence
                        for task_id in all_tasks {
                            commands.entity(task_id).insert(SequenceMember(sequence_entity));
                        }
                    }
                }
            }
            (None, Some(excess_items)) => {
                // Existing excess handling logic unchanged
                let pickup_task = commands.spawn((
                    TaskBundle::new(
                        event.crafter,
                        event.position,
                        TaskAction::Pickup(Some(excess_items.clone())),
                        Priority::Medium,
                    ),
                )).id();
                
                if let Some((receiver_entity, receiver_pos)) = find_closest_storage_receiver(
                    (event.position.x, event.position.y),
                    excess_items,
                    &buildings,
                    &recipes
                ) {
                    let dropoff_task = commands.spawn((
                        TaskBundle::new(
                            receiver_entity,
                            receiver_pos,
                            TaskAction::Dropoff(None),
                            Priority::Medium,
                        ),
                    )).id();
                    
                    let sequence_entity = commands.spawn(
                        TaskSequenceBundle::new(
                            vec![pickup_task, dropoff_task],
                            Priority::Medium
                        )
                    ).id();
                    
                    commands.entity(pickup_task).insert(SequenceMember(sequence_entity));
                    commands.entity(dropoff_task).insert(SequenceMember(sequence_entity));
                } else {
                    commands.entity(pickup_task).despawn();
                }
            }
            _ => {}
        }
    }
}

fn calculate_supply_plan(
    requester_pos: (i32, i32),
    needed_items: &HashMap<ItemName, u32>,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    let mut remaining_needs = needed_items.clone();
    let mut supply_plan = Vec::new();
    
    while !remaining_needs.is_empty() {
        let mut best_contribution: Option<(Entity, Position, HashMap<ItemName, u32>)> = None;
        let mut best_distance = i32::MAX;
        
        // Find the closest building that can contribute something we still need
        for (entity, pos, inventory, inv_type, _) in buildings.iter() {
            if inv_type.0 != InventoryTypes::Storage && inv_type.0 != InventoryTypes::Sender {
                continue;
            }
            
            let mut contribution = HashMap::new();
            
            // Calculate what this building can actually contribute
            for (item_name, &still_needed) in remaining_needs.iter() {
                let available = inventory.get_item_quantity(item_name);
                if available > 0 {
                    let can_contribute = available.min(still_needed);
                    contribution.insert(item_name.clone(), can_contribute);
                }
            }
            
            if contribution.is_empty() {
                continue;
            }
            
            let distance = manhattan_distance_coords(requester_pos, (pos.x, pos.y));
            if distance < best_distance {
                best_distance = distance;
                best_contribution = Some((entity, *pos, contribution));
            }
        }
        
        // Add the best contribution to our plan and update remaining needs
        if let Some((entity, pos, contribution)) = best_contribution {
            // Subtract this contribution from remaining needs
            for (item_name, contributed_amount) in &contribution {
                if let Some(still_needed) = remaining_needs.get_mut(item_name) {
                    *still_needed = still_needed.saturating_sub(*contributed_amount);
                    if *still_needed == 0 {
                        remaining_needs.remove(item_name);
                    }
                }
            }
            
            supply_plan.push((entity, pos, contribution));
        } else {
            // No building can contribute anything we need
            break;
        }
    }
    
    supply_plan
}

fn find_closest_storage_receiver(
    position: (i32, i32),
    _items: &HashMap<ItemName, u32>, // Future: filter by item compatibility
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    _recipes: &Res<RecipeRegistry>,
) -> Option<(Entity, Position)> {
    let mut closest_building = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, pos, inv, inv_type, _) in buildings.iter() {
        if inv_type.0 == InventoryTypes::Storage {
            let distance = manhattan_distance_coords((pos.x, pos.y), position);
            if distance < closest_distance && !inv.is_full() {
                closest_building = Some((entity, *pos));
                closest_distance = distance;
            }
        }
    }
    
    closest_building
}

pub fn derive_worker_state_from_sequences(
    mut workers: Query<(Entity, &mut AssignedSequence, &mut WorkerState), With<Worker>>,
    mut sequences: Query<&mut TaskSequence>,
    tasks: Query<Entity, With<Task>>, // Add this parameter for validation
) {
    for (worker_entity, mut assigned_sequence, mut worker_state) in workers.iter_mut() {
        let new_state = match assigned_sequence.0 {
            None => WorkerState::Idle,
            Some(sequence_entity) => {
                match sequences.get_mut(sequence_entity) {
                    Ok(mut sequence) => {
                        // Validate and advance past invalid tasks
                        sequence.validate_and_advance(&tasks);
                        
                        // Check completion with validation
                        if sequence.is_complete_with_validation(&tasks) {
                            // Sequence is complete or invalid, clean up assignment
                            assigned_sequence.0 = None;
                            WorkerState::Idle
                        } else {
                            WorkerState::Working
                        }
                    }
                    Err(_) => {
                        // Sequence entity was despawned, clean up assignment
                        assigned_sequence.0 = None;
                        WorkerState::Idle
                    }
                }
            }
        };
        
        // Only update if state has changed to avoid unnecessary change detection
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