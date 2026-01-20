use super::components::{AssignedWorker, Task, TaskAction, TaskSequence, TaskStatus, TaskTarget};
use crate::{
    grid::{Grid, Position},
    materials::{
        items::{Cargo, InventoryAccess, OutputPort, StoragePort},
        request_transfer_specific_items, ItemTransferRequestEvent,
    },
    systems::NetworkConnectivity,
    workers::{calculate_path, AssignedSequence, Worker, WorkerArrivedEvent, WorkerPath},
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
                *task_status = TaskStatus::Completed;
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
