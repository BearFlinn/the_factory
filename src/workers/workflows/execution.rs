use super::components::{
    StepTarget, WaitingForItems, Workflow, WorkflowAction, WorkflowAssignment,
};
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
use std::collections::{HashMap, HashSet};

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

fn resolve_step_target(
    step: &super::components::WorkflowStep,
    building_set: &HashSet<Entity>,
    positions: &Query<&Position>,
    names: &Query<&Name>,
    round_robin: &mut HashMap<(Entity, usize), usize>,
    workflow_entity: Entity,
    step_index: usize,
) -> Option<Entity> {
    match &step.target {
        StepTarget::Specific(entity) => {
            if building_set.contains(entity) && positions.get(*entity).is_ok() {
                Some(*entity)
            } else {
                None
            }
        }
        StepTarget::ByType(type_name) => {
            let mut candidates: Vec<(Entity, &Position)> = building_set
                .iter()
                .filter_map(|&entity| {
                    let name = names.get(entity).ok()?;
                    if name.as_str() != type_name {
                        return None;
                    }
                    let pos = positions.get(entity).ok()?;
                    Some((entity, pos))
                })
                .collect();

            if candidates.is_empty() {
                return None;
            }

            candidates.sort_by(|a, b| {
                a.1.x
                    .cmp(&b.1.x)
                    .then_with(|| a.1.y.cmp(&b.1.y))
                    .then_with(|| a.0.cmp(&b.0))
            });

            let key = (workflow_entity, step_index);
            let counter = round_robin.entry(key).or_insert(0);
            let idx = *counter % candidates.len();
            *counter += 1;

            Some(candidates[idx].0)
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
    names: Query<&Name>,
    network: Res<NetworkConnectivity>,
    grid: Res<Grid>,
) {
    let mut round_robin: HashMap<(Entity, usize), usize> = HashMap::new();

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

        let Some(target_entity) = resolve_step_target(
            step,
            &workflow.building_set,
            &positions,
            &names,
            &mut round_robin,
            assignment.workflow,
            assignment.current_step,
        ) else {
            assignment.current_step = workflow.next_step(assignment.current_step);
            continue;
        };

        assignment.resolved_target = Some(target_entity);

        let Ok(target_pos) = positions.get(target_entity) else {
            assignment.current_step = workflow.next_step(assignment.current_step);
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

        let target = match assignment.resolved_target {
            Some(entity) => entity,
            None => match &step.target {
                StepTarget::Specific(entity) => *entity,
                StepTarget::ByType(_) => continue,
            },
        };

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

        assignment.resolved_target = None;
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

        let target = match assignment.resolved_target {
            Some(entity) => entity,
            None => match &step.target {
                StepTarget::Specific(entity) => *entity,
                StepTarget::ByType(_) => continue,
            },
        };

        if let WorkflowAction::Pickup(filter) = &step.action {
            let available =
                get_available_items_at(target, &output_ports, &storage_ports, &input_ports);
            let items = compute_pickup_items(&available, filter.as_ref());

            if !items.is_empty() {
                commands.entity(worker_entity).remove::<WaitingForItems>();
                request_transfer_specific_items(target, worker_entity, items, &mut transfer_events);
                assignment.resolved_target = None;
                assignment.current_step = workflow.next_step(assignment.current_step);
            }
        }
    }
}

pub fn cleanup_invalid_workflow_refs(
    mut commands: Commands,
    mut workers: Query<(Entity, &mut WorkflowAssignment)>,
    mut workflows: Query<&mut Workflow>,
    positions: Query<&Position>,
) {
    for mut workflow in &mut workflows {
        workflow
            .building_set
            .retain(|entity| positions.get(*entity).is_ok());
    }

    for (worker_entity, mut assignment) in &mut workers {
        let Ok(workflow) = workflows.get(assignment.workflow) else {
            commands
                .entity(worker_entity)
                .remove::<WorkflowAssignment>();
            continue;
        };

        if let Some(resolved) = assignment.resolved_target {
            if positions.get(resolved).is_err() {
                assignment.resolved_target = None;
            }
        }

        if let Some(step) = workflow.steps.get(assignment.current_step) {
            match &step.target {
                StepTarget::Specific(entity) => {
                    if positions.get(*entity).is_err() {
                        assignment.current_step = workflow.next_step(assignment.current_step);
                    }
                }
                StepTarget::ByType(_) => {}
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
    use crate::workers::workflows::components::WorkflowStep;
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

    #[test]
    fn resolve_step_target_specific_in_set() {
        let mut app = App::new();
        let building = app
            .world_mut()
            .spawn((Position { x: 5, y: 5 }, Name::new("Smelter")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let mut building_set = HashSet::new();
        building_set.insert(building);
        let step = WorkflowStep {
            target: StepTarget::Specific(building),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();
                let result = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                assert_eq!(result, Some(building));
            })
            .unwrap();
    }

    #[test]
    fn resolve_step_target_specific_not_in_set() {
        let mut app = App::new();
        let building = app
            .world_mut()
            .spawn((Position { x: 5, y: 5 }, Name::new("Smelter")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let building_set = HashSet::new();
        let step = WorkflowStep {
            target: StepTarget::Specific(building),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();
                let result = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                assert!(result.is_none());
            })
            .unwrap();
    }

    #[test]
    fn resolve_step_target_by_type_round_robin() {
        let mut app = App::new();
        let smelter_a = app
            .world_mut()
            .spawn((Position { x: 2, y: 0 }, Name::new("Smelter")))
            .id();
        let smelter_b = app
            .world_mut()
            .spawn((Position { x: 5, y: 0 }, Name::new("Smelter")))
            .id();
        let smelter_c = app
            .world_mut()
            .spawn((Position { x: 8, y: 0 }, Name::new("Smelter")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let mut building_set = HashSet::new();
        building_set.insert(smelter_a);
        building_set.insert(smelter_b);
        building_set.insert(smelter_c);
        let step = WorkflowStep {
            target: StepTarget::ByType("Smelter".to_string()),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();
                let r1 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                let r2 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                let r3 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                let r4 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );

                assert_eq!(r1, Some(smelter_a));
                assert_eq!(r2, Some(smelter_b));
                assert_eq!(r3, Some(smelter_c));
                assert_eq!(r4, Some(smelter_a));
            })
            .unwrap();
    }

    #[test]
    fn resolve_step_target_by_type_no_match() {
        let mut app = App::new();
        let drill = app
            .world_mut()
            .spawn((Position { x: 5, y: 5 }, Name::new("Mining Drill")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let mut building_set = HashSet::new();
        building_set.insert(drill);
        let step = WorkflowStep {
            target: StepTarget::ByType("Smelter".to_string()),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();
                let result = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                assert!(result.is_none());
            })
            .unwrap();
    }

    #[test]
    fn resolve_step_target_round_robin_independent_per_step() {
        let mut app = App::new();
        let smelter_a = app
            .world_mut()
            .spawn((Position { x: 2, y: 0 }, Name::new("Smelter")))
            .id();
        let smelter_b = app
            .world_mut()
            .spawn((Position { x: 5, y: 0 }, Name::new("Smelter")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let mut building_set = HashSet::new();
        building_set.insert(smelter_a);
        building_set.insert(smelter_b);
        let step = WorkflowStep {
            target: StepTarget::ByType("Smelter".to_string()),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();

                let r_step0 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                let r_step1 = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    1,
                );

                assert_eq!(r_step0, Some(smelter_a));
                assert_eq!(r_step1, Some(smelter_a));

                let r_step0_again = resolve_step_target(
                    &step,
                    &building_set,
                    &positions,
                    &names,
                    &mut rr,
                    workflow_entity,
                    0,
                );
                assert_eq!(r_step0_again, Some(smelter_b));
            })
            .unwrap();
    }

    #[test]
    fn resolve_step_target_by_type_single_building_always_returns_it() {
        let mut app = App::new();
        let smelter = app
            .world_mut()
            .spawn((Position { x: 3, y: 3 }, Name::new("Smelter")))
            .id();
        let workflow_entity = app.world_mut().spawn_empty().id();
        let mut building_set = HashSet::new();
        building_set.insert(smelter);
        let step = WorkflowStep {
            target: StepTarget::ByType("Smelter".to_string()),
            action: WorkflowAction::Pickup(None),
        };

        app.world_mut()
            .run_system_once(move |positions: Query<&Position>, names: Query<&Name>| {
                let mut rr = HashMap::new();
                for _ in 0..5 {
                    let result = resolve_step_target(
                        &step,
                        &building_set,
                        &positions,
                        &names,
                        &mut rr,
                        workflow_entity,
                        0,
                    );
                    assert_eq!(result, Some(smelter));
                }
            })
            .unwrap();
    }
}
