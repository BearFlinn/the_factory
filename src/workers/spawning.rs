use std::collections::VecDeque;
use bevy::prelude::*;
use crate::{
    grid::Position, materials::items::{Inventory, InventoryType, InventoryTypes}, structures::ComputeConsumer, workers::{tasks::TaskAction, WorkerPath}
};

#[derive(Component)]
pub struct Worker;

#[derive(Component)]
pub struct Speed {
    pub value: f32,
}

#[derive(Component, PartialEq, Debug)]
pub enum WorkerState {
    Idle,
    Working,
}

pub struct WorkerTaskInfo {
    pub task: Entity,
    pub target: Entity,
    pub position: Position,
    pub action: TaskAction,
}

#[derive(Component)]
pub struct WorkerTasks (pub VecDeque<WorkerTaskInfo>);

#[derive(Bundle)]
pub struct WorkerBundle {
    pub worker: Worker,
    pub speed: Speed,
    pub position: Position,
    pub path: WorkerPath,
    pub tasks: WorkerTasks,
    pub state: WorkerState,
    pub inventory: Inventory,
    pub inventory_type: InventoryType,
    pub compute_consumer: ComputeConsumer,
    pub sprite: Sprite,
    pub transform: Transform,
}

// Update the impl to include the new component:
impl WorkerBundle {
    pub fn new(spawn_position: Vec2) -> Self {
        WorkerBundle {
            worker: Worker,
            speed: Speed { value: 250.0 },
            position: Position { x: spawn_position.x as i32, y: spawn_position.y as i32 },
            path: WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            tasks: WorkerTasks(VecDeque::new()),
            state: WorkerState::Idle,
            inventory: Inventory::new(20),
            inventory_type: InventoryType(InventoryTypes::Carrier),
            compute_consumer: ComputeConsumer { amount: 10 },
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}