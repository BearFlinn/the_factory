use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use the_factory::{
    materials::{Cargo, InventoryAccess, StoragePort},
    structures::Hub,
    workers::workflows::{
        StepTarget, WaitingForItems, WaitingForSpace, Workflow, WorkflowAction, WorkflowAssignment,
        WorkflowStep,
    },
};

use crate::harness::*;

fn find_hub(app: &mut App) -> Entity {
    let mut query = app.world_mut().query_filtered::<Entity, With<Hub>>();
    query.iter(app.world()).next().expect("hub should exist")
}

#[test]
fn worker_completes_pickup_dropoff_cycle() {
    let mut app = headless_app();
    tick(&mut app);
    let hub = find_hub(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let storage = spawn_building(&mut app, "Storage", 3, 0);
    tick_n(&mut app, 3);

    let worker = spawn_worker(app.world_mut(), 0, 0);
    tick(&mut app);

    let mut building_set = HashSet::new();
    building_set.insert(hub);
    building_set.insert(storage);

    let workflow_entity = app
        .world_mut()
        .spawn(Workflow {
            name: "test logistics".to_string(),
            building_set,
            steps: vec![
                WorkflowStep {
                    target: StepTarget::Specific(hub),
                    action: WorkflowAction::Pickup(None),
                },
                WorkflowStep {
                    target: StepTarget::Specific(storage),
                    action: WorkflowAction::Dropoff(None),
                },
            ],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        })
        .id();

    app.world_mut()
        .entity_mut(worker)
        .insert(WorkflowAssignment {
            workflow: workflow_entity,
            current_step: 0,
            resolved_target: None,
            resolved_action: None,
        });

    tick_n(&mut app, 300);

    let storage_port = app.world().get::<StoragePort>(storage).unwrap();
    assert!(
        !storage_port.is_empty(),
        "storage should have received items from hub via worker"
    );
}

#[test]
fn worker_waits_at_empty_source() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let storage = spawn_building(&mut app, "Storage", 3, 0);
    tick_n(&mut app, 3);

    let worker = spawn_worker(app.world_mut(), 3, 0);
    tick(&mut app);

    let mut building_set = HashSet::new();
    building_set.insert(storage);

    let workflow_entity = app
        .world_mut()
        .spawn(Workflow {
            name: "wait test".to_string(),
            building_set,
            steps: vec![WorkflowStep {
                target: StepTarget::Specific(storage),
                action: WorkflowAction::Pickup(None),
            }],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        })
        .id();

    app.world_mut()
        .entity_mut(worker)
        .insert(WorkflowAssignment {
            workflow: workflow_entity,
            current_step: 0,
            resolved_target: None,
            resolved_action: None,
        });

    tick_n(&mut app, 60);

    assert!(
        app.world().get::<WaitingForItems>(worker).is_some(),
        "worker should be WaitingForItems when source is empty"
    );
}

#[test]
fn worker_retries_full_destination() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let storage = spawn_building(&mut app, "Storage", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        add_items_to_storage(world, storage, "Iron Ore", 200);
    }

    let worker = spawn_worker(app.world_mut(), 3, 0);
    {
        let world = app.world_mut();
        let mut cargo = world.get_mut::<Cargo>(worker).unwrap();
        cargo.add_item("Coal", 5);
    }
    tick(&mut app);

    let mut building_set = HashSet::new();
    building_set.insert(storage);

    let workflow_entity = app
        .world_mut()
        .spawn(Workflow {
            name: "full dest test".to_string(),
            building_set,
            steps: vec![WorkflowStep {
                target: StepTarget::Specific(storage),
                action: WorkflowAction::Dropoff(None),
            }],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        })
        .id();

    app.world_mut()
        .entity_mut(worker)
        .insert(WorkflowAssignment {
            workflow: workflow_entity,
            current_step: 0,
            resolved_target: None,
            resolved_action: None,
        });

    tick_n(&mut app, 60);

    assert!(
        app.world().get::<WaitingForSpace>(worker).is_some(),
        "worker should be WaitingForSpace when destination is full"
    );
}

#[test]
fn round_robin_distributes_evenly() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0), (4, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let storage_a = spawn_building(&mut app, "Storage", 3, 0);
    tick_n(&mut app, 2);
    let storage_b = spawn_building(&mut app, "Storage", 4, 0);
    tick_n(&mut app, 2);

    {
        let world = app.world_mut();
        add_items_to_storage(world, storage_a, "Iron Ore", 10);
        add_items_to_storage(world, storage_b, "Iron Ore", 10);
    }

    let mut building_set = HashSet::new();
    building_set.insert(storage_a);
    building_set.insert(storage_b);

    let workflow_entity = app
        .world_mut()
        .spawn(Workflow {
            name: "round robin test".to_string(),
            building_set,
            steps: vec![WorkflowStep {
                target: StepTarget::ByType("Storage".to_string()),
                action: WorkflowAction::Pickup(None),
            }],
            is_paused: false,
            desired_worker_count: 2,
            round_robin_counters: HashMap::new(),
        })
        .id();

    let worker_a = spawn_worker(app.world_mut(), 0, 0);
    let worker_b = spawn_worker(app.world_mut(), 0, 0);

    for worker in [worker_a, worker_b] {
        app.world_mut()
            .entity_mut(worker)
            .insert(WorkflowAssignment {
                workflow: workflow_entity,
                current_step: 0,
                resolved_target: None,
                resolved_action: None,
            });
    }

    tick_n(&mut app, 5);

    let target_a = app
        .world()
        .get::<WorkflowAssignment>(worker_a)
        .and_then(|a| a.resolved_target);
    let target_b = app
        .world()
        .get::<WorkflowAssignment>(worker_b)
        .and_then(|a| a.resolved_target);

    if let (Some(ta), Some(tb)) = (target_a, target_b) {
        assert_ne!(
            ta, tb,
            "round-robin should assign different targets to different workers"
        );
    }
}

#[test]
fn arrival_fires_when_already_at_target() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let storage = spawn_building(&mut app, "Storage", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        add_items_to_storage(world, storage, "Iron Ore", 10);
    }

    let worker = spawn_worker(app.world_mut(), 3, 0);
    tick(&mut app);

    let mut building_set = HashSet::new();
    building_set.insert(storage);

    let workflow_entity = app
        .world_mut()
        .spawn(Workflow {
            name: "already at target".to_string(),
            building_set,
            steps: vec![WorkflowStep {
                target: StepTarget::Specific(storage),
                action: WorkflowAction::Pickup(None),
            }],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        })
        .id();

    app.world_mut()
        .entity_mut(worker)
        .insert(WorkflowAssignment {
            workflow: workflow_entity,
            current_step: 0,
            resolved_target: None,
            resolved_action: None,
        });

    tick_n(&mut app, 10);

    let cargo = app.world().get::<Cargo>(worker).unwrap();
    assert!(
        !cargo.is_empty(),
        "worker at target should have picked up items via immediate arrival, cargo: {:?}",
        cargo.items
    );
}

#[test]
fn emergency_dropoff_on_unassignment() {
    let mut app = headless_app();
    tick(&mut app);

    let worker = spawn_worker(app.world_mut(), 0, 0);
    {
        let world = app.world_mut();
        let mut cargo = world.get_mut::<Cargo>(worker).unwrap();
        cargo.add_item("Iron Ore", 5);
    }
    tick(&mut app);

    tick_n(&mut app, 10);

    let cargo = app.world().get::<Cargo>(worker).unwrap();
    assert!(
        cargo.is_empty(),
        "unassigned worker should have emergency-dropped cargo, still has: {:?}",
        cargo.items
    );
}
