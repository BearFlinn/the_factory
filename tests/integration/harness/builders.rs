use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::collections::VecDeque;

use the_factory::{
    grid::{Grid, Position},
    materials::{Cargo, InputPort, InventoryAccess, OutputPort, StoragePort},
    structures::{BuildingRegistry, ComputeConsumer},
    systems::{NetworkChangedEvent, NetworkConnectivity},
    workers::{Speed, Worker, WorkerPath},
};

pub fn ensure_grid_coordinates(world: &mut World, coords: &[(i32, i32)]) {
    let mut grid = world.resource_mut::<Grid>();
    for &(x, y) in coords {
        grid.add_coordinate(x, y);
    }
}

pub fn connect_to_network(world: &mut World, coords: &[(i32, i32)]) {
    let mut network = world.resource_mut::<NetworkConnectivity>();
    for &(x, y) in coords {
        network.add_connected_cell(x, y);
        network.add_core_network_cell(x, y);
    }
}

pub fn spawn_worker(world: &mut World, x: i32, y: i32) -> Entity {
    let grid = world.resource::<Grid>();
    let world_pos = grid.grid_to_world_coordinates(x, y);

    world
        .spawn((
            Worker,
            Speed { value: 250.0 },
            Position { x, y },
            WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            Cargo::new(20),
            ComputeConsumer { amount: 10 },
            Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            Transform::from_xyz(world_pos.x, world_pos.y, 1.5),
        ))
        .id()
}

#[allow(clippy::cast_precision_loss)]
pub fn spawn_building(app: &mut App, name: &str, x: i32, y: i32) -> Entity {
    let world_pos = {
        let grid = app.world().resource::<Grid>();
        grid.grid_to_world_coordinates(x, y)
    };

    let name_owned = name.to_string();
    let entity = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, registry: Res<BuildingRegistry>| {
                registry
                    .spawn_building(&mut commands, &name_owned, x, y, world_pos)
                    .unwrap_or_else(|| {
                        panic!("failed to spawn building '{name_owned}' - not found in registry")
                    })
            },
        )
        .unwrap();

    app.world_mut().write_message(NetworkChangedEvent);

    entity
}

pub fn add_items_to_input(world: &mut World, entity: Entity, item: &str, qty: u32) {
    if let Some(mut port) = world.get_mut::<InputPort>(entity) {
        port.add_item(item, qty);
    } else {
        panic!("entity {entity:?} has no InputPort - cannot add items");
    }
}

pub fn add_items_to_output(world: &mut World, entity: Entity, item: &str, qty: u32) {
    if let Some(mut port) = world.get_mut::<OutputPort>(entity) {
        port.add_item(item, qty);
    } else {
        panic!("entity {entity:?} has no OutputPort - cannot add items");
    }
}

pub fn add_items_to_storage(world: &mut World, entity: Entity, item: &str, qty: u32) {
    if let Some(mut port) = world.get_mut::<StoragePort>(entity) {
        port.add_item(item, qty);
    } else {
        panic!("entity {entity:?} has no StoragePort - cannot add items");
    }
}
