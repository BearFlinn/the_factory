use super::components::{
    AssignedWorker, SequenceMember, Task, TaskAction, TaskBundle, TaskSequence, TaskSequenceBundle,
    TaskStatus, TaskTarget,
};
use crate::{
    grid::{Grid, Position},
    materials::{
        items::{Cargo, InventoryAccess, OutputPort, StoragePort},
        request_transfer_specific_items, ItemTransferRequestEvent,
    },
    systems::NetworkConnectivity,
    workers::{
        calculate_path,
        dispatcher::{DispatcherConfig, WorkerDispatcher},
        manhattan_distance_coords, AssignedSequence, Worker, WorkerArrivedEvent, WorkerPath,
        WorkerStateComputation,
    },
};
use bevy::prelude::*;
use std::collections::HashMap;

pub fn process_worker_sequences(
    mut workers: Query<(Entity, &Position, &mut AssignedSequence, &mut WorkerPath), With<Worker>>,
    mut sequences: Query<&mut TaskSequence>,
    mut tasks: Query<(&Position, &mut TaskStatus, &TaskTarget), With<Task>>,
    task_targets: Query<Entity>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    for (worker_entity, worker_position, mut assigned_sequence, mut worker_path) in &mut workers {
        let Some(sequence_entity) = assigned_sequence.0 else {
            continue;
        };

        if !validate_and_process_sequence(
            sequence_entity,
            &mut assigned_sequence,
            &mut sequences,
            &mut tasks,
            &task_targets,
        ) {
            continue;
        }

        let Ok(sequence) = sequences.get(sequence_entity) else {
            continue;
        };
        let Some(current_task_entity) = sequence.current_task() else {
            continue;
        };
        let Ok((task_position, mut task_status, _)) = tasks.get_mut(current_task_entity) else {
            continue;
        };

        if *task_status == TaskStatus::Pending || *task_status == TaskStatus::Queued {
            *task_status = TaskStatus::InProgress;
        }

        if worker_path.current_target.is_some() || !worker_path.waypoints.is_empty() {
            continue;
        }

        initiate_pathfinding_or_complete_task(
            worker_entity,
            *worker_position,
            *task_position,
            &mut worker_path,
            &mut assigned_sequence,
            &mut sequences,
            &grid,
            &network,
            &mut arrival_events,
        );
    }
}

fn validate_and_process_sequence(
    sequence_entity: Entity,
    assigned_sequence: &mut AssignedSequence,
    sequences: &mut Query<&mut TaskSequence>,
    tasks: &mut Query<(&Position, &mut TaskStatus, &TaskTarget), With<Task>>,
    task_targets: &Query<Entity>,
) -> bool {
    let Ok(mut sequence) = sequences.get_mut(sequence_entity) else {
        assigned_sequence.0 = None;
        return false;
    };

    if sequence.is_complete() {
        assigned_sequence.0 = None;
        return false;
    }

    let Some(current_task_entity) = sequence.current_task() else {
        assigned_sequence.0 = None;
        return false;
    };

    let Ok((_, mut task_status, task_target)) = tasks.get_mut(current_task_entity) else {
        if sequence.advance_to_next().is_none() {
            assigned_sequence.0 = None;
        }
        return false;
    };

    if task_targets.get(task_target.0).is_err() {
        *task_status = TaskStatus::Completed;
        if sequence.advance_to_next().is_none() {
            assigned_sequence.0 = None;
        }
        return false;
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn initiate_pathfinding_or_complete_task(
    worker_entity: Entity,
    worker_position: Position,
    task_position: Position,
    worker_path: &mut WorkerPath,
    assigned_sequence: &mut AssignedSequence,
    sequences: &mut Query<&mut TaskSequence>,
    grid: &Grid,
    network: &NetworkConnectivity,
    arrival_events: &mut EventWriter<WorkerArrivedEvent>,
) {
    let worker_pos = (worker_position.x, worker_position.y);
    let target_pos = (task_position.x, task_position.y);

    if let Some(path) = calculate_path(worker_pos, target_pos, network, grid) {
        worker_path.waypoints = path;
        worker_path.current_target = worker_path.waypoints.pop_front();

        if worker_path.current_target.is_none() && worker_path.waypoints.is_empty() {
            arrival_events.send(WorkerArrivedEvent {
                worker: worker_entity,
                position: target_pos,
            });
        }
    } else if let Some(sequence_entity) = assigned_sequence.0 {
        if let Ok(mut sequence) = sequences.get_mut(sequence_entity) {
            if sequence.advance_to_next().is_none() {
                assigned_sequence.0 = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_sequence_task_arrivals(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    mut workers: Query<(&mut AssignedSequence, &Cargo), With<Worker>>,
    mut sequences: Query<(&mut TaskSequence, &mut AssignedWorker)>,
    mut tasks: Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
    task_targets: Query<Entity>,
    output_ports: Query<&OutputPort>,
    storage_ports: Query<&StoragePort>,
    mut transfer_requests: EventWriter<ItemTransferRequestEvent>,
    mut delivery_completed_events: EventWriter<super::components::LogisticsDeliveryCompletedEvent>,
) {
    for event in arrival_events.read() {
        let Ok((mut worker_assigned_sequence, worker_cargo)) = workers.get_mut(event.worker) else {
            continue;
        };

        let Some(sequence_entity) = worker_assigned_sequence.0 else {
            continue;
        };

        if !validate_arrival_context(
            event,
            sequence_entity,
            &mut worker_assigned_sequence,
            &mut sequences,
            &mut tasks,
            &task_targets,
        ) {
            continue;
        }

        let Ok((mut sequence, mut sequence_assigned_worker)) = sequences.get_mut(sequence_entity)
        else {
            continue;
        };
        let Some(current_task_entity) = sequence.current_task() else {
            continue;
        };
        let Ok((_, task_action, task_target, mut task_status)) = tasks.get_mut(current_task_entity)
        else {
            continue;
        };

        if let TaskAction::Pickup(Some(items)) = task_action {
            let available = get_available_items(task_target.0, &output_ports, &storage_ports);
            let has_any_requested = items
                .keys()
                .any(|item| available.get(item).copied().unwrap_or(0) > 0);

            if !has_any_requested {
                cancel_sequence(&mut sequence, &mut tasks);
                worker_assigned_sequence.0 = None;
                sequence_assigned_worker.0 = None;
                continue;
            }
        }

        if let TaskAction::Dropoff(Some(_)) = task_action {
            if worker_cargo.is_empty() {
                *task_status = TaskStatus::Completed;
                if sequence.advance_to_next().is_none() {
                    worker_assigned_sequence.0 = None;
                    sequence_assigned_worker.0 = None;
                }
                continue;
            }
        }

        execute_task_action(
            event.worker,
            task_action,
            task_target,
            worker_cargo,
            &mut transfer_requests,
        );

        if let TaskAction::Dropoff(Some(_)) = task_action {
            let cargo_items = worker_cargo.get_all_items();
            if !cargo_items.is_empty() {
                delivery_completed_events.send(
                    super::components::LogisticsDeliveryCompletedEvent {
                        building: task_target.0,
                        items: cargo_items,
                    },
                );
            }
        }

        *task_status = TaskStatus::Completed;

        if sequence.advance_to_next().is_none() {
            worker_assigned_sequence.0 = None;
            sequence_assigned_worker.0 = None;
        }
    }
}

fn get_available_items(
    entity: Entity,
    output_ports: &Query<&OutputPort>,
    storage_ports: &Query<&StoragePort>,
) -> HashMap<String, u32> {
    if let Ok(port) = output_ports.get(entity) {
        return port.items().iter().map(|(k, &v)| (k.clone(), v)).collect();
    }
    if let Ok(port) = storage_ports.get(entity) {
        return port.items().iter().map(|(k, &v)| (k.clone(), v)).collect();
    }
    HashMap::new()
}

fn cancel_sequence(
    sequence: &mut TaskSequence,
    tasks: &mut Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
) {
    for task_entity in &sequence.tasks {
        if let Ok((_, _, _, mut status)) = tasks.get_mut(*task_entity) {
            *status = TaskStatus::Completed;
        }
    }
    sequence.current_index = sequence.tasks.len();
}

fn validate_arrival_context(
    event: &WorkerArrivedEvent,
    sequence_entity: Entity,
    worker_assigned_sequence: &mut AssignedSequence,
    sequences: &mut Query<(&mut TaskSequence, &mut AssignedWorker)>,
    tasks: &mut Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
    task_targets: &Query<Entity>,
) -> bool {
    let Ok((mut sequence, mut sequence_assigned_worker)) = sequences.get_mut(sequence_entity)
    else {
        worker_assigned_sequence.0 = None;
        return false;
    };

    let Some(current_task_entity) = sequence.current_task() else {
        worker_assigned_sequence.0 = None;
        sequence_assigned_worker.0 = None;
        return false;
    };

    let Ok((task_position, _, task_target, mut task_status)) = tasks.get_mut(current_task_entity)
    else {
        if sequence.advance_to_next().is_none() {
            worker_assigned_sequence.0 = None;
            sequence_assigned_worker.0 = None;
        }
        return false;
    };

    if task_targets.get(task_target.0).is_err() {
        *task_status = TaskStatus::Completed;
        if sequence.advance_to_next().is_none() {
            worker_assigned_sequence.0 = None;
            sequence_assigned_worker.0 = None;
        }
        return false;
    }

    let task_pos = (task_position.x, task_position.y);
    if event.position != task_pos {
        return false;
    }

    true
}

fn execute_task_action(
    worker_entity: Entity,
    task_action: &TaskAction,
    task_target: &TaskTarget,
    worker_cargo: &Cargo,
    transfer_requests: &mut EventWriter<ItemTransferRequestEvent>,
) {
    match task_action {
        TaskAction::Pickup(items) => {
            if let Some(items) = items {
                request_transfer_specific_items(
                    task_target.0,
                    worker_entity,
                    items.clone(),
                    transfer_requests,
                );
            } else {
                let empty_items = std::collections::HashMap::new();
                request_transfer_specific_items(
                    task_target.0,
                    worker_entity,
                    empty_items,
                    transfer_requests,
                );
            }
        }
        TaskAction::Dropoff(items) => {
            if let Some(items) = items {
                request_transfer_specific_items(
                    worker_entity,
                    task_target.0,
                    items.clone(),
                    transfer_requests,
                );
            } else {
                let cargo_items = worker_cargo.get_all_items();
                if !cargo_items.is_empty() {
                    request_transfer_specific_items(
                        worker_entity,
                        task_target.0,
                        cargo_items,
                        transfer_requests,
                    );
                }
            }
        }
    }
}

/// Checks for task chaining opportunities when a worker becomes idle.
/// If a nearby pending request exists that meets the criteria, assigns it directly
/// instead of having the worker return to the hub.
#[allow(clippy::too_many_arguments)]
pub fn try_chain_nearby_tasks(
    mut commands: Commands,
    mut workers: Query<(Entity, &Position, &mut AssignedSequence, &Cargo), With<Worker>>,
    mut dispatcher: ResMut<WorkerDispatcher>,
    config: Res<DispatcherConfig>,
    mut delivery_started_events: EventWriter<super::components::LogisticsDeliveryStartedEvent>,
) {
    // Find workers that just became idle (no assignment, empty cargo)
    let idle_workers: Vec<(Entity, Position)> = workers
        .iter()
        .filter(|(_, _, assigned, cargo)| assigned.is_idle() && cargo.is_empty())
        .map(|(entity, pos, _, _)| (entity, *pos))
        .collect();

    for (worker_entity, worker_pos) in idle_workers {
        // Find the best nearby request to chain
        let worker_grid_pos = (worker_pos.x, worker_pos.y);
        let mut best_request_idx = None;
        let mut best_distance = i32::MAX;

        for (idx, request) in dispatcher.pending_requests.iter().enumerate() {
            // Must meet urgency threshold
            if request.urgency < config.chain_urgency_threshold {
                continue;
            }

            // Must be within distance threshold
            let source_pos = (request.source_pos.x, request.source_pos.y);
            let distance = manhattan_distance_coords(worker_grid_pos, source_pos);

            if distance > config.chain_distance_threshold {
                continue;
            }

            // Take the closest qualifying request
            if distance < best_distance {
                best_distance = distance;
                best_request_idx = Some(idx);
            }
        }

        // Chain the task if we found one
        if let Some(idx) = best_request_idx {
            let request = dispatcher.pending_requests.remove(idx);

            // Create the pickup/dropoff sequence
            let pickup_task = commands
                .spawn(TaskBundle::new(
                    request.source,
                    request.source_pos,
                    TaskAction::Pickup(Some(request.items.clone())),
                    request.priority.clone(),
                ))
                .id();

            let dropoff_task = commands
                .spawn(TaskBundle::new(
                    request.destination,
                    request.destination_pos,
                    TaskAction::Dropoff(Some(request.items.clone())),
                    request.priority.clone(),
                ))
                .id();

            let sequence_entity = commands
                .spawn(TaskSequenceBundle::new(
                    vec![pickup_task, dropoff_task],
                    request.priority.clone(),
                ))
                .id();

            commands
                .entity(pickup_task)
                .insert(SequenceMember(sequence_entity));
            commands
                .entity(dropoff_task)
                .insert(SequenceMember(sequence_entity));

            // Assign worker to sequence
            commands
                .entity(sequence_entity)
                .insert(AssignedWorker(Some(worker_entity)));

            if let Ok((_, _, mut assigned_sequence, _)) = workers.get_mut(worker_entity) {
                assigned_sequence.0 = Some(sequence_entity);
            }

            // Emit delivery started event
            delivery_started_events.send(super::components::LogisticsDeliveryStartedEvent {
                building: request.destination,
                items: request.items.clone(),
            });

            println!(
                "Task chain: Worker {worker_entity:?} at ({}, {}) chained to request at ({}, {}) (urgency: {:.2}, distance: {})",
                worker_pos.x, worker_pos.y, request.source_pos.x, request.source_pos.y, request.urgency, best_distance
            );
        }
    }
}
