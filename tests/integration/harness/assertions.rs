use bevy::prelude::*;

use the_factory::{
    grid::Position,
    materials::{InputPort, InventoryAccess, OutputPort, StoragePort},
    systems::Operational,
};

pub fn assert_inventory_has(world: &World, entity: Entity, item: &str, expected_qty: u32) {
    let mut found = false;
    let mut actual_qty = 0;
    let mut port_type = "";

    if let Some(port) = world.get::<OutputPort>(entity) {
        actual_qty = port.get_item_quantity(item);
        port_type = "OutputPort";
        found = true;
    }
    if !found {
        if let Some(port) = world.get::<InputPort>(entity) {
            actual_qty = port.get_item_quantity(item);
            port_type = "InputPort";
            found = true;
        }
    }
    if !found {
        if let Some(port) = world.get::<StoragePort>(entity) {
            actual_qty = port.get_item_quantity(item);
            port_type = "StoragePort";
            found = true;
        }
    }

    assert!(
        found,
        "entity {entity:?} has no inventory port (OutputPort, InputPort, or StoragePort)"
    );
    assert_eq!(
        actual_qty, expected_qty,
        "entity {entity:?} {port_type}: expected {expected_qty}x '{item}', found {actual_qty}"
    );
}

pub fn assert_inventory_empty(world: &World, entity: Entity) {
    let mut has_port = false;

    if let Some(port) = world.get::<OutputPort>(entity) {
        has_port = true;
        assert!(
            port.is_empty(),
            "entity {entity:?} OutputPort not empty: {:?}",
            port.items
        );
    }
    if let Some(port) = world.get::<InputPort>(entity) {
        has_port = true;
        assert!(
            port.is_empty(),
            "entity {entity:?} InputPort not empty: {:?}",
            port.items
        );
    }
    if let Some(port) = world.get::<StoragePort>(entity) {
        has_port = true;
        assert!(
            port.is_empty(),
            "entity {entity:?} StoragePort not empty: {:?}",
            port.items
        );
    }

    assert!(has_port, "entity {entity:?} has no inventory port");
}

pub fn assert_worker_at(world: &World, worker: Entity, x: i32, y: i32) {
    let pos = world
        .get::<Position>(worker)
        .unwrap_or_else(|| panic!("entity {worker:?} has no Position component"));
    assert_eq!(
        (pos.x, pos.y),
        (x, y),
        "worker {worker:?}: expected position ({x}, {y}), found ({}, {})",
        pos.x,
        pos.y
    );
}

pub fn assert_operational(world: &World, entity: Entity) {
    let operational = world
        .get::<Operational>(entity)
        .unwrap_or_else(|| panic!("entity {entity:?} has no Operational component"));
    assert!(
        operational.get_status(),
        "entity {entity:?} expected operational, but conditions: {:?}",
        operational.0
    );
}

pub fn assert_not_operational(world: &World, entity: Entity) {
    let operational = world
        .get::<Operational>(entity)
        .unwrap_or_else(|| panic!("entity {entity:?} has no Operational component"));
    assert!(
        !operational.get_status(),
        "entity {entity:?} expected NOT operational, but all conditions are met"
    );
}

pub fn assert_has_component<T: Component>(world: &World, entity: Entity) {
    assert!(
        world.get::<T>(entity).is_some(),
        "entity {entity:?} missing expected component {}",
        std::any::type_name::<T>()
    );
}
