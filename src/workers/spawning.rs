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

#[derive(Component)]
pub struct AssignedSequence(pub Option<Entity>);

pub trait WorkerStateComputation {
    fn is_idle(&self) -> bool;
    fn is_working(&self) -> bool;
}

impl WorkerStateComputation for AssignedSequence {
    fn is_idle(&self) -> bool {
        self.0.is_none()
    }

    fn is_working(&self) -> bool {
        self.0.is_some()
    }
}

#[derive(Bundle)]
pub struct WorkerBundle {
    pub worker: Worker,
    pub speed: Speed,
    pub position: Position,
    pub path: WorkerPath,
    pub assigned_sequence: AssignedSequence,
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
            cargo: Cargo::new(20),
            compute_consumer: ComputeConsumer { amount: 10 },
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigned_sequence_is_idle_returns_true_when_none() {
        let sequence = AssignedSequence(None);

        assert!(sequence.is_idle());
    }

    #[test]
    fn assigned_sequence_is_idle_returns_false_when_some() {
        let sequence = AssignedSequence(Some(Entity::from_raw(1)));

        assert!(!sequence.is_idle());
    }

    #[test]
    fn assigned_sequence_is_working_returns_true_when_some() {
        let sequence = AssignedSequence(Some(Entity::from_raw(42)));

        assert!(sequence.is_working());
    }

    #[test]
    fn assigned_sequence_is_working_returns_false_when_none() {
        let sequence = AssignedSequence(None);

        assert!(!sequence.is_working());
    }

    #[test]
    fn is_idle_and_is_working_are_mutually_exclusive() {
        let idle_sequence = AssignedSequence(None);
        let working_sequence = AssignedSequence(Some(Entity::from_raw(1)));

        assert!(idle_sequence.is_idle() && !idle_sequence.is_working());
        assert!(!working_sequence.is_idle() && working_sequence.is_working());
    }
}
