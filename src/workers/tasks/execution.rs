use super::components::{AssignedWorker, Task, TaskAction, TaskSequence, TaskStatus, TaskTarget};
use crate::{
    grid::{Grid, Position},
    materials::{
        items::{Cargo, InventoryAccess},
        request_transfer_specific_items, ItemTransferRequestEvent,
    },
    systems::NetworkConnectivity,
    workers::{
        calculate_path, AssignedSequence, Worker, WorkerArrivedEvent, WorkerPath, WorkerState,
    },
};
use bevy::prelude::*;

pub fn process_worker_sequences(
    mut workers: Query<
        (
            Entity,
            &Transform,
            &mut AssignedSequence,
            &mut WorkerPath,
            &mut WorkerState,
        ),
        With<Worker>,
    >,
    mut sequences: Query<&mut TaskSequence>,
    mut tasks: Query<(&Position, &mut TaskStatus, &TaskTarget), With<Task>>,
    task_targets: Query<Entity>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    for (worker_entity, transform, mut assigned_sequence, mut worker_path, mut worker_state) in
        &mut workers
    {
        let Some(sequence_entity) = assigned_sequence.0 else {
            continue;
        };

        if !validate_and_process_sequence(
            sequence_entity,
            &mut assigned_sequence,
            &mut worker_state,
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
            transform,
            *task_position,
            &mut worker_path,
            &mut assigned_sequence,
            &mut worker_state,
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
    worker_state: &mut WorkerState,
    sequences: &mut Query<&mut TaskSequence>,
    tasks: &mut Query<(&Position, &mut TaskStatus, &TaskTarget), With<Task>>,
    task_targets: &Query<Entity>,
) -> bool {
    let Ok(mut sequence) = sequences.get_mut(sequence_entity) else {
        assigned_sequence.0 = None;
        *worker_state = WorkerState::Idle;
        return false;
    };

    if sequence.is_complete() {
        assigned_sequence.0 = None;
        *worker_state = WorkerState::Idle;
        return false;
    }

    let Some(current_task_entity) = sequence.current_task() else {
        assigned_sequence.0 = None;
        *worker_state = WorkerState::Idle;
        return false;
    };

    let Ok((_, mut task_status, task_target)) = tasks.get_mut(current_task_entity) else {
        if sequence.advance_to_next().is_none() {
            assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
        }
        return false;
    };

    if task_targets.get(task_target.0).is_err() {
        *task_status = TaskStatus::Completed;
        if sequence.advance_to_next().is_none() {
            assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
        }
        return false;
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn initiate_pathfinding_or_complete_task(
    worker_entity: Entity,
    transform: &Transform,
    task_position: Position,
    worker_path: &mut WorkerPath,
    assigned_sequence: &mut AssignedSequence,
    worker_state: &mut WorkerState,
    sequences: &mut Query<&mut TaskSequence>,
    grid: &Grid,
    network: &NetworkConnectivity,
    arrival_events: &mut EventWriter<WorkerArrivedEvent>,
) {
    let Some(worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate())
    else {
        return;
    };

    let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
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
                *worker_state = WorkerState::Idle;
            }
        }
    }
}

pub fn handle_sequence_task_arrivals(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    mut workers: Query<(&mut AssignedSequence, &Cargo, &mut WorkerState), With<Worker>>,
    mut sequences: Query<(&mut TaskSequence, &mut AssignedWorker)>,
    mut tasks: Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
    task_targets: Query<Entity>,
    mut transfer_requests: EventWriter<ItemTransferRequestEvent>,
) {
    for event in arrival_events.read() {
        let Ok((mut worker_assigned_sequence, worker_cargo, mut worker_state)) =
            workers.get_mut(event.worker)
        else {
            continue;
        };

        let Some(sequence_entity) = worker_assigned_sequence.0 else {
            continue;
        };

        if !validate_arrival_context(
            event,
            sequence_entity,
            &mut worker_assigned_sequence,
            &mut worker_state,
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

        execute_task_action(
            event.worker,
            task_action,
            task_target,
            worker_cargo,
            &mut transfer_requests,
        );

        *task_status = TaskStatus::Completed;

        if sequence.advance_to_next().is_none() {
            *worker_state = WorkerState::Idle;
            worker_assigned_sequence.0 = None;
            sequence_assigned_worker.0 = None;
        }
    }
}

fn validate_arrival_context(
    event: &WorkerArrivedEvent,
    sequence_entity: Entity,
    worker_assigned_sequence: &mut AssignedSequence,
    worker_state: &mut WorkerState,
    sequences: &mut Query<(&mut TaskSequence, &mut AssignedWorker)>,
    tasks: &mut Query<(&Position, &TaskAction, &TaskTarget, &mut TaskStatus), With<Task>>,
    task_targets: &Query<Entity>,
) -> bool {
    let Ok((mut sequence, mut sequence_assigned_worker)) = sequences.get_mut(sequence_entity)
    else {
        worker_assigned_sequence.0 = None;
        *worker_state = WorkerState::Idle;
        return false;
    };

    let Some(current_task_entity) = sequence.current_task() else {
        worker_assigned_sequence.0 = None;
        *worker_state = WorkerState::Idle;
        sequence_assigned_worker.0 = None;
        return false;
    };

    let Ok((task_position, _, task_target, mut task_status)) = tasks.get_mut(current_task_entity)
    else {
        if sequence.advance_to_next().is_none() {
            worker_assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
            sequence_assigned_worker.0 = None;
        }
        return false;
    };

    if task_targets.get(task_target.0).is_err() {
        *task_status = TaskStatus::Completed;
        if sequence.advance_to_next().is_none() {
            worker_assigned_sequence.0 = None;
            *worker_state = WorkerState::Idle;
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
                // If no specific items requested, this is a special case
                // The transfer system will handle finding items from the source's port
                // For now, we create a transfer request with empty items which will be validated
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
                // Transfer all items from worker cargo
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
