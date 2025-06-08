use std::collections::VecDeque;
use bevy::prelude::*;
use crate::{
    materials::items::{Inventory, InventoryType, InventoryTypes}, structures::ComputeConsumer, workers::{WorkerPath, WorkerTask}
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
    pub task: WorkerTask,
    pub path: WorkerPath,
    pub inventory: Inventory,
    pub inventory_type: InventoryType,
    pub compute_consumer: ComputeConsumer,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl WorkerBundle {
    pub fn new(spawn_position: Vec2) -> Self {
        WorkerBundle {
            worker: Worker,
            speed: Speed { value: 250.0 },
            task: WorkerTask { destination: None, task: None },
            path: WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            inventory: Inventory::new(20),
            inventory_type: InventoryType(InventoryTypes::Carrier),
            compute_consumer: ComputeConsumer { amount: 10 },
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}