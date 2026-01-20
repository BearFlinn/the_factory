use super::components::{
    AssignedWorker, InterruptType, PendingEmergencyDropoff, Priority, SequenceMember, Task,
    TaskAction, TaskBundle, TaskSequence, TaskSequenceBundle, TaskStatus, WorkerInterruptEvent,
};
use crate::{
    grid::Position,
    materials::{Cargo, InventoryAccess, StoragePort},
    structures::Building,
    workers::{
        dispatcher::{DispatcherConfig, WorkerDispatcher},
        manhattan_distance_coords, AssignedSequence, Worker, WorkerPath, WorkerStateComputation,
    },
};
use bevy::prelude::*;

/// Batch-optimized worker assignment system.
/// Considers all available workers and unassigned sequences together,
/// optimizing assignments globally rather than greedily assigning one at a time.
pub fn assign_available_sequences_to_workers(
    mut sequences: Query<(Entity, &mut AssignedWorker, &TaskSequence, &Priority)>,
    mut workers: Query<(Entity, &Position, &mut AssignedSequence, &Cargo), With<Worker>>,
    tasks: Query<&Position, With<Task>>,
    dispatcher: Res<WorkerDispatcher>,
    config: Res<DispatcherConfig>,
) {
    cleanup_orphaned_assignments(&mut sequences, &mut workers);

    // Collect all available workers
    let mut available_workers: Vec<(Entity, Position)> = workers
        .iter()
        .filter(|(_, _, assigned_sequence, cargo)| assigned_sequence.is_idle() && cargo.is_empty())
        .map(|(entity, pos, _, _)| (entity, *pos))
        .collect();

    if available_workers.is_empty() {
        return;
    }

    // Sort workers: pooled workers first (known location at hub), then by distance from origin
    available_workers.sort_by(|(entity_a, pos_a), (entity_b, pos_b)| {
        let a_pooled = dispatcher.is_worker_pooled(*entity_a);
        let b_pooled = dispatcher.is_worker_pooled(*entity_b);
        match (a_pooled, b_pooled) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                // Sort by distance from origin (hub)
                let dist_a = pos_a.x.abs() + pos_a.y.abs();
                let dist_b = pos_b.x.abs() + pos_b.y.abs();
                dist_a.cmp(&dist_b)
            }
        }
    });

    // Collect all unassigned sequences with their task positions
    let mut unassigned_sequences: Vec<(Entity, Position, Priority)> = Vec::new();

    for (sequence_entity, assigned_worker, sequence, priority) in sequences.iter() {
        if assigned_worker.0.is_some() || sequence.is_complete() {
            continue;
        }

        let Some(current_task_entity) = sequence.current_task() else {
            continue;
        };

        let Ok(task_position) = tasks.get(current_task_entity) else {
            continue;
        };

        unassigned_sequences.push((sequence_entity, *task_position, priority.clone()));
    }

    if unassigned_sequences.is_empty() {
        return;
    }

    // Sort sequences by priority (Critical first)
    unassigned_sequences.sort_by(|a, b| {
        let priority_ord = |p: &Priority| match p {
            Priority::Critical => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
        };
        priority_ord(&a.2).cmp(&priority_ord(&b.2))
    });

    // Batch assignment: for each priority level, find optimal matches
    let assignments = compute_batch_assignments(&available_workers, &unassigned_sequences, &config);

    // Apply assignments
    let mut assignments_made = 0;
    for (worker_entity, sequence_entity) in assignments {
        if let Ok((_, mut assigned_worker, _, priority)) = sequences.get_mut(sequence_entity) {
            if let Ok((_, _, mut worker_assigned_sequence, _)) = workers.get_mut(worker_entity) {
                assigned_worker.0 = Some(worker_entity);
                worker_assigned_sequence.0 = Some(sequence_entity);
                assignments_made += 1;

                println!(
                    "Batch assigned worker {worker_entity:?} to {priority:?} priority sequence {sequence_entity:?}"
                );
            }
        }
    }

    if assignments_made > 0 {
        println!("Batch assignment: {assignments_made} workers assigned");
    }
}

/// Computes optimal worker-to-sequence assignments using a cost-based approach.
/// Returns a list of (worker, sequence) entity pairs.
fn compute_batch_assignments(
    available_workers: &[(Entity, Position)],
    unassigned_sequences: &[(Entity, Position, Priority)],
    _config: &DispatcherConfig,
) -> Vec<(Entity, Entity)> {
    let mut assignments = Vec::new();
    let mut assigned_workers: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    let mut assigned_sequences: std::collections::HashSet<Entity> =
        std::collections::HashSet::new();

    // Group sequences by priority
    let mut priority_groups: std::collections::HashMap<u8, Vec<(Entity, Position)>> =
        std::collections::HashMap::new();

    for (seq_entity, pos, priority) in unassigned_sequences {
        let priority_key = match priority {
            Priority::Critical => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
        };
        priority_groups
            .entry(priority_key)
            .or_default()
            .push((*seq_entity, *pos));
    }

    // Process each priority level in order
    for priority_key in [0u8, 1, 2, 3] {
        let Some(sequences_at_priority) = priority_groups.get(&priority_key) else {
            continue;
        };

        // Get remaining available workers
        let remaining_workers: Vec<(Entity, Position)> = available_workers
            .iter()
            .filter(|(e, _)| !assigned_workers.contains(e))
            .copied()
            .collect();

        if remaining_workers.is_empty() {
            break;
        }

        // Get remaining sequences at this priority
        let remaining_sequences: Vec<(Entity, Position)> = sequences_at_priority
            .iter()
            .filter(|(e, _)| !assigned_sequences.contains(e))
            .copied()
            .collect();

        if remaining_sequences.is_empty() {
            continue;
        }

        // Compute cost-optimal assignment for this priority batch
        let batch_assignments =
            compute_minimum_cost_assignment(&remaining_workers, &remaining_sequences);

        for (worker_entity, sequence_entity) in batch_assignments {
            assigned_workers.insert(worker_entity);
            assigned_sequences.insert(sequence_entity);
            assignments.push((worker_entity, sequence_entity));
        }
    }

    assignments
}

/// Computes minimum-cost assignment between workers and sequences.
/// Uses a greedy approach optimized for typical game scenarios.
/// For small counts, this performs well. For large counts (>50), consider Hungarian algorithm.
fn compute_minimum_cost_assignment(
    workers: &[(Entity, Position)],
    sequences: &[(Entity, Position)],
) -> Vec<(Entity, Entity)> {
    if workers.is_empty() || sequences.is_empty() {
        return Vec::new();
    }

    // Build cost matrix: cost[worker_idx][sequence_idx] = distance
    let mut costs: Vec<Vec<i32>> = Vec::with_capacity(workers.len());
    for (_, worker_pos) in workers {
        let mut row = Vec::with_capacity(sequences.len());
        for (_, seq_pos) in sequences {
            let distance =
                manhattan_distance_coords((worker_pos.x, worker_pos.y), (seq_pos.x, seq_pos.y));
            row.push(distance);
        }
        costs.push(row);
    }

    // Greedy minimum-cost matching
    // For each iteration, find the (worker, sequence) pair with minimum cost
    // and assign them, then remove both from consideration
    let mut assignments = Vec::new();
    let mut used_workers = vec![false; workers.len()];
    let mut used_sequences = vec![false; sequences.len()];
    let max_assignments = workers.len().min(sequences.len());

    for _ in 0..max_assignments {
        let mut best_cost = i32::MAX;
        let mut best_worker_idx = 0;
        let mut best_seq_idx = 0;

        for (worker_idx, worker_used) in used_workers.iter().enumerate() {
            if *worker_used {
                continue;
            }
            for (seq_idx, seq_used) in used_sequences.iter().enumerate() {
                if *seq_used {
                    continue;
                }
                if costs[worker_idx][seq_idx] < best_cost {
                    best_cost = costs[worker_idx][seq_idx];
                    best_worker_idx = worker_idx;
                    best_seq_idx = seq_idx;
                }
            }
        }

        if best_cost == i32::MAX {
            break;
        }

        used_workers[best_worker_idx] = true;
        used_sequences[best_seq_idx] = true;
        assignments.push((workers[best_worker_idx].0, sequences[best_seq_idx].0));
    }

    assignments
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
    workers: &mut Query<(Entity, &Position, &mut AssignedSequence, &Cargo), With<Worker>>,
) {
    let mut orphaned_sequences = Vec::new();

    for (sequence_entity, assigned_worker, _, _) in sequences.iter() {
        if let Some(worker_entity) = assigned_worker.0 {
            if let Ok((_, _, worker_assigned_sequence, _)) = workers.get(worker_entity) {
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

pub fn handle_worker_interrupts(
    mut commands: Commands,
    mut interrupt_events: EventReader<WorkerInterruptEvent>,
    mut workers: Query<(&mut AssignedSequence, &mut WorkerPath), With<Worker>>,
    mut sequences: Query<&mut AssignedWorker>,
) {
    for event in interrupt_events.read() {
        let Ok((mut worker_assigned_sequence, mut worker_path)) = workers.get_mut(event.worker)
        else {
            println!(
                "WorkerInterrupt: Worker entity {:?} not found",
                event.worker
            );
            continue;
        };

        if let Some(old_sequence_entity) = worker_assigned_sequence.0 {
            if let Ok(mut old_assigned_worker) = sequences.get_mut(old_sequence_entity) {
                old_assigned_worker.0 = None;
            }
        }

        worker_path.waypoints.clear();
        worker_path.current_target = None;

        match &event.interrupt_type {
            InterruptType::ReplaceSequence(new_sequence_entity) => {
                if let Ok(mut new_assigned_worker) = sequences.get_mut(*new_sequence_entity) {
                    worker_assigned_sequence.0 = Some(*new_sequence_entity);
                    new_assigned_worker.0 = Some(event.worker);

                    println!(
                        "WorkerInterrupt: Worker {:?} assigned to sequence {:?}",
                        event.worker, new_sequence_entity
                    );
                } else {
                    worker_assigned_sequence.0 = None;

                    println!(
                        "WorkerInterrupt: New sequence {:?} not found, worker {:?} set to idle",
                        new_sequence_entity, event.worker
                    );
                }
            }

            InterruptType::ReplaceTasks(new_tasks, priority) => {
                if new_tasks.is_empty() {
                    worker_assigned_sequence.0 = None;

                    println!(
                        "WorkerInterrupt: Empty task list, worker {:?} set to idle",
                        event.worker
                    );
                } else {
                    let new_sequence_entity = commands
                        .spawn(TaskSequenceBundle::new(new_tasks.clone(), priority.clone()))
                        .id();

                    worker_assigned_sequence.0 = Some(new_sequence_entity);

                    commands
                        .entity(new_sequence_entity)
                        .insert(AssignedWorker(Some(event.worker)));

                    for &task_entity in new_tasks {
                        commands
                            .entity(task_entity)
                            .insert(SequenceMember(new_sequence_entity));
                    }

                    println!(
                        "WorkerInterrupt: Worker {:?} assigned to new sequence {:?} with {} tasks",
                        event.worker,
                        new_sequence_entity,
                        new_tasks.len()
                    );
                }
            }

            InterruptType::ClearAssignment => {
                worker_assigned_sequence.0 = None;

                println!(
                    "WorkerInterrupt: Worker {:?} assignment cleared",
                    event.worker
                );
            }
        }
    }
}

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
            println!("Debug: Cleared assignments for {worker_count} workers");
        }
    }
}

pub fn emergency_dropoff_idle_workers(
    mut commands: Commands,
    workers: Query<
        (
            Entity,
            &Position,
            &AssignedSequence,
            &Cargo,
            Option<&PendingEmergencyDropoff>,
        ),
        With<Worker>,
    >,
    storage_buildings: Query<(Entity, &Position, &StoragePort), With<Building>>,
    mut interrupt_events: EventWriter<WorkerInterruptEvent>,
) {
    for (worker_entity, worker_pos, assigned_sequence, worker_cargo, pending_dropoff) in
        workers.iter()
    {
        if !assigned_sequence.is_idle() || worker_cargo.is_empty() || pending_dropoff.is_some() {
            continue;
        }

        let worker_grid_pos = (worker_pos.x, worker_pos.y);
        let nearest_storage = find_nearest_available_storage(worker_grid_pos, &storage_buildings);

        if let Some((storage_entity, storage_pos)) = nearest_storage {
            let worker_items = worker_cargo.get_all_items();

            if !worker_items.is_empty() {
                let pickup_task = commands
                    .spawn(TaskBundle::new(
                        worker_entity,
                        *worker_pos,
                        TaskAction::Pickup(Some(worker_items.clone())),
                        Priority::Medium,
                    ))
                    .id();

                let dropoff_task = commands
                    .spawn(TaskBundle::new(
                        storage_entity,
                        storage_pos,
                        TaskAction::Dropoff(Some(worker_items)),
                        Priority::Medium,
                    ))
                    .id();

                commands
                    .entity(worker_entity)
                    .insert(PendingEmergencyDropoff);

                interrupt_events.send(WorkerInterruptEvent {
                    worker: worker_entity,
                    interrupt_type: InterruptType::ReplaceTasks(
                        vec![pickup_task, dropoff_task],
                        Priority::Medium,
                    ),
                });

                println!("Emergency: Created dropoff sequence for worker {worker_entity:?} → storage {storage_entity:?}");
            }
        } else {
            commands
                .entity(worker_entity)
                .insert(PendingEmergencyDropoff);
        }
    }
}

fn find_nearest_available_storage(
    worker_pos: (i32, i32),
    storage_buildings: &Query<(Entity, &Position, &StoragePort), With<Building>>,
) -> Option<(Entity, Position)> {
    let mut nearest_storage = None;
    let mut closest_distance = i32::MAX;

    for (entity, position, storage_port) in storage_buildings.iter() {
        if storage_port.is_full() {
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

pub fn cleanup_emergency_dropoff_markers(
    mut commands: Commands,
    workers: Query<
        (Entity, &Position, &AssignedSequence, &Cargo),
        (With<Worker>, With<PendingEmergencyDropoff>),
    >,
    storage_buildings: Query<(Entity, &Position, &StoragePort), With<Building>>,
) {
    for (worker_entity, worker_pos, assigned_sequence, cargo) in workers.iter() {
        if !assigned_sequence.is_idle() {
            continue;
        }

        if cargo.is_empty() {
            commands
                .entity(worker_entity)
                .remove::<PendingEmergencyDropoff>();
        } else {
            let worker_grid_pos = (worker_pos.x, worker_pos.y);
            if find_nearest_available_storage(worker_grid_pos, &storage_buildings).is_some() {
                commands
                    .entity(worker_entity)
                    .remove::<PendingEmergencyDropoff>();
            }
        }
    }
}
