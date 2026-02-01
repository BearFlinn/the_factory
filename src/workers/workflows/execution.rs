use super::components::{WaitingForItems, Workflow, WorkflowAction, WorkflowAssignment};
use crate::{
    grid::{Grid, Position},
    materials::{
        request_transfer_specific_items, Cargo, InputPort, InventoryAccess,
        ItemTransferRequestEvent, OutputPort, StoragePort,
    },
    systems::NetworkConnectivity,
    workers::{pathfinding::calculate_path, Worker, WorkerArrivedEvent, WorkerPath},
};
use bevy::prelude::*;
use std::collections::HashMap;

fn get_available_items_at(
    target: Entity,
    output_ports: &Query<&OutputPort>,
    storage_ports: &Query<&StoragePort>,
    input_ports: &Query<&InputPort>,
) -> HashMap<String, u32> {
    if let Ok(port) = output_ports.get(target) {
        if !port.is_empty() {
            return port.get_all_items();
        }
    }
    if let Ok(port) = storage_ports.get(target) {
        if !port.is_empty() {
            return port.get_all_items();
        }
    }
    if let Ok(port) = input_ports.get(target) {
        if !port.is_empty() {
            return port.get_all_items();
        }
    }
    HashMap::new()
}

fn compute_pickup_items(
    available: &HashMap<String, u32>,
    filter: Option<&HashMap<String, u32>>,
) -> HashMap<String, u32> {
    match filter {
        None => available.clone(),
        Some(requested) => {
            let mut result = HashMap::new();
            for (item, &requested_qty) in requested {
                if let Some(&available_qty) = available.get(item) {
                    let qty = available_qty.min(requested_qty);
                    if qty > 0 {
                        result.insert(item.clone(), qty);
                    }
                }
            }
            result
        }
    }
}

fn compute_dropoff_items(
    cargo_items: &HashMap<String, u32>,
    filter: Option<&HashMap<String, u32>>,
) -> HashMap<String, u32> {
    match filter {
        None => cargo_items.clone(),
        Some(requested) => {
            let mut result = HashMap::new();
            for (item, &requested_qty) in requested {
                if let Some(&cargo_qty) = cargo_items.get(item) {
                    let qty = cargo_qty.min(requested_qty);
                    if qty > 0 {
                        result.insert(item.clone(), qty);
                    }
                }
            }
            result
        }
    }
}

pub fn process_workflow_workers(
    mut workers: Query<
        (Entity, &mut WorkflowAssignment, &Position, &mut WorkerPath),
        (With<Worker>, Without<WaitingForItems>),
    >,
    workflows: Query<&Workflow>,
    positions: Query<&Position>,
    network: Res<NetworkConnectivity>,
    grid: Res<Grid>,
) {
    for (_worker_entity, mut assignment, worker_pos, mut path) in &mut workers {
        let Ok(workflow) = workflows.get(assignment.workflow) else {
            continue;
        };

        if workflow.is_paused {
            continue;
        }

        if workflow.steps.is_empty() {
            continue;
        }

        let Some(step) = workflow.steps.get(assignment.current_step) else {
            continue;
        };

        if path.current_target.is_some() {
            continue;
        }

        let Ok(target_pos) = positions.get(step.target) else {
            continue;
        };

        let start = (worker_pos.x, worker_pos.y);
        let end = (target_pos.x, target_pos.y);

        if let Some(mut waypoints) = calculate_path(start, end, &network, &grid) {
            let first = waypoints.pop_front();
            path.waypoints = waypoints;
            path.current_target = first;
        } else {
            assignment.current_step = workflow.next_step(assignment.current_step);
        }
    }
}

pub fn handle_workflow_arrivals(
    mut events: MessageReader<WorkerArrivedEvent>,
    mut workers: Query<(&mut WorkflowAssignment, &Cargo), With<Worker>>,
    workflows: Query<&Workflow>,
    output_ports: Query<&OutputPort>,
    storage_ports: Query<&StoragePort>,
    input_ports: Query<&InputPort>,
    mut transfer_events: MessageWriter<ItemTransferRequestEvent>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut assignment, cargo)) = workers.get_mut(event.worker) else {
            continue;
        };

        let Ok(workflow) = workflows.get(assignment.workflow) else {
            continue;
        };

        if workflow.steps.is_empty() {
            continue;
        }

        let Some(step) = workflow.steps.get(assignment.current_step) else {
            continue;
        };

        let target = step.target;

        match &step.action {
            WorkflowAction::Pickup(filter) => {
                let available =
                    get_available_items_at(target, &output_ports, &storage_ports, &input_ports);
                let items = compute_pickup_items(&available, filter.as_ref());

                if items.is_empty() {
                    commands
                        .entity(event.worker)
                        .insert(WaitingForItems::default());
                    continue;
                }

                request_transfer_specific_items(target, event.worker, items, &mut transfer_events);
            }
            WorkflowAction::Dropoff(filter) => {
                let cargo_items = cargo.get_all_items();
                let items = compute_dropoff_items(&cargo_items, filter.as_ref());

                request_transfer_specific_items(event.worker, target, items, &mut transfer_events);
            }
        }

        assignment.current_step = workflow.next_step(assignment.current_step);
    }
}

pub fn recheck_waiting_workers(
    mut commands: Commands,
    time: Res<Time>,
    mut workers: Query<(Entity, &mut WaitingForItems, &mut WorkflowAssignment), With<Worker>>,
    workflows: Query<&Workflow>,
    output_ports: Query<&OutputPort>,
    storage_ports: Query<&StoragePort>,
    input_ports: Query<&InputPort>,
    mut transfer_events: MessageWriter<ItemTransferRequestEvent>,
) {
    for (worker_entity, mut waiting, mut assignment) in &mut workers {
        waiting.timer.tick(time.delta());

        if !waiting.timer.just_finished() {
            continue;
        }

        let Ok(workflow) = workflows.get(assignment.workflow) else {
            continue;
        };

        let Some(step) = workflow.steps.get(assignment.current_step) else {
            continue;
        };

        let target = step.target;

        if let WorkflowAction::Pickup(filter) = &step.action {
            let available =
                get_available_items_at(target, &output_ports, &storage_ports, &input_ports);
            let items = compute_pickup_items(&available, filter.as_ref());

            if !items.is_empty() {
                commands.entity(worker_entity).remove::<WaitingForItems>();
                request_transfer_specific_items(target, worker_entity, items, &mut transfer_events);
                assignment.current_step = workflow.next_step(assignment.current_step);
            }
        }
    }
}

pub fn cleanup_invalid_workflow_refs(
    mut commands: Commands,
    mut workers: Query<(Entity, &mut WorkflowAssignment)>,
    workflows: Query<&Workflow>,
    positions: Query<&Position>,
) {
    for (worker_entity, mut assignment) in &mut workers {
        let Ok(workflow) = workflows.get(assignment.workflow) else {
            commands
                .entity(worker_entity)
                .remove::<WorkflowAssignment>();
            continue;
        };

        if let Some(step) = workflow.steps.get(assignment.current_step) {
            if positions.get(step.target).is_err() {
                assignment.current_step = workflow.next_step(assignment.current_step);
            }
        }
    }
}

pub fn emergency_dropoff_unassigned_workers(
    workers: Query<(Entity, &Cargo, &Position), (With<Worker>, Without<WorkflowAssignment>)>,
    storage_ports: Query<(Entity, &Position), With<StoragePort>>,
    mut transfer_events: MessageWriter<ItemTransferRequestEvent>,
) {
    for (worker_entity, cargo, worker_pos) in &workers {
        if cargo.is_empty() {
            continue;
        }

        let mut nearest: Option<(Entity, i32)> = None;

        for (storage_entity, storage_pos) in &storage_ports {
            let dist = (worker_pos.x - storage_pos.x).abs() + (worker_pos.y - storage_pos.y).abs();
            match nearest {
                Some((_, best_dist)) if dist < best_dist => {
                    nearest = Some((storage_entity, dist));
                }
                None => {
                    nearest = Some((storage_entity, dist));
                }
                _ => {}
            }
        }

        if let Some((storage_entity, _)) = nearest {
            let items = cargo.get_all_items();
            request_transfer_specific_items(
                worker_entity,
                storage_entity,
                items,
                &mut transfer_events,
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn get_available_items_empty_returns_empty() {
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();

        app.world_mut()
            .run_system_once(
                move |output_ports: Query<&OutputPort>,
                      storage_ports: Query<&StoragePort>,
                      input_ports: Query<&InputPort>| {
                    let result =
                        get_available_items_at(target, &output_ports, &storage_ports, &input_ports);
                    assert!(result.is_empty());
                },
            )
            .unwrap();
    }

    #[test]
    fn compute_pickup_items_none_filter_returns_all() {
        let mut available = HashMap::new();
        available.insert("iron_ore".to_string(), 10);
        available.insert("copper_ore".to_string(), 5);

        let result = compute_pickup_items(&available, None);

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("iron_ore"), Some(&10));
        assert_eq!(result.get("copper_ore"), Some(&5));
    }

    #[test]
    fn compute_pickup_items_some_filter_caps_at_available() {
        let mut available = HashMap::new();
        available.insert("iron_ore".to_string(), 3);

        let mut filter = HashMap::new();
        filter.insert("iron_ore".to_string(), 10);

        let result = compute_pickup_items(&available, Some(&filter));

        assert_eq!(result.get("iron_ore"), Some(&3));
    }

    #[test]
    fn compute_pickup_items_some_filter_missing_item_excluded() {
        let mut available = HashMap::new();
        available.insert("iron_ore".to_string(), 5);

        let mut filter = HashMap::new();
        filter.insert("copper_ore".to_string(), 10);

        let result = compute_pickup_items(&available, Some(&filter));

        assert!(result.is_empty());
    }

    #[test]
    fn compute_dropoff_items_none_filter_returns_all() {
        let mut cargo_items = HashMap::new();
        cargo_items.insert("iron_plate".to_string(), 8);
        cargo_items.insert("copper_plate".to_string(), 4);

        let result = compute_dropoff_items(&cargo_items, None);

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("iron_plate"), Some(&8));
        assert_eq!(result.get("copper_plate"), Some(&4));
    }

    #[test]
    fn compute_dropoff_items_some_filter_caps_at_cargo() {
        let mut cargo_items = HashMap::new();
        cargo_items.insert("iron_plate".to_string(), 3);

        let mut filter = HashMap::new();
        filter.insert("iron_plate".to_string(), 10);

        let result = compute_dropoff_items(&cargo_items, Some(&filter));

        assert_eq!(result.get("iron_plate"), Some(&3));
    }

    #[test]
    fn compute_dropoff_items_some_filter_empty_cargo_returns_empty() {
        let cargo_items: HashMap<String, u32> = HashMap::new();

        let mut filter = HashMap::new();
        filter.insert("iron_plate".to_string(), 5);

        let result = compute_dropoff_items(&cargo_items, Some(&filter));

        assert!(result.is_empty());
    }
}
