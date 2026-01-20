use crate::{
    grid::Position, materials::items::Cargo, structures::ComputeConsumer, workers::WorkerPath,
};
use bevy::prelude::*;
use std::collections::VecDeque;

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

#[derive(Component)]
pub struct AssignedSequence(pub Option<Entity>);

#[derive(Bundle)]
pub struct WorkerBundle {
    pub worker: Worker,
    pub speed: Speed,
    pub position: Position,
    pub path: WorkerPath,
    pub assigned_sequence: AssignedSequence,
    pub state: WorkerState,
    pub cargo: Cargo,
    pub compute_consumer: ComputeConsumer,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl WorkerBundle {
    pub fn new(spawn_position: Vec2) -> Self {
        WorkerBundle {
            worker: Worker,
            speed: Speed { value: 250.0 },
            #[allow(clippy::cast_possible_truncation)]
            position: Position {
                x: spawn_position.x as i32,
                y: spawn_position.y as i32,
            },
            path: WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            assigned_sequence: AssignedSequence(None),
            state: WorkerState::Idle,
            cargo: Cargo::new(20),
            compute_consumer: ComputeConsumer { amount: 10 },
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}
