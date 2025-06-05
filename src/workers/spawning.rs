use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position}, items::{Inventory, InventoryType, InventoryTypes}, structures::{Building, BuildingId, Hub, MultiCellBuilding, MINING_DRILL}, systems::Operational, workers::WorkerPath
};

#[derive(Component)]
pub struct Worker;

#[derive(Component)]
pub struct Speed {
    pub value: f32,
}

#[derive(Bundle)]
pub struct WorkerBundle {
    pub worker: Worker,
    pub speed: Speed,
    pub path: WorkerPath,
    pub inventory: Inventory,
    pub inventory_type: InventoryType,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl WorkerBundle {
    pub fn new(spawn_position: Vec2) -> Self {
        WorkerBundle {
            worker: Worker,
            speed: Speed { value: 250.0 }, // pixels per second
            path: WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            inventory: Inventory::new(20),
            inventory_type: InventoryType(InventoryTypes::Carrier),
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}