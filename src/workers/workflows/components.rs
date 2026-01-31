use bevy::prelude::*;
use std::collections::HashMap;

use crate::materials::ItemName;

#[derive(Clone, Debug)]
pub enum WorkflowAction {
    Pickup(Option<HashMap<ItemName, u32>>),
    Dropoff(Option<HashMap<ItemName, u32>>),
}

#[derive(Clone, Debug)]
pub struct WorkflowStep {
    pub target: Entity,
    pub action: WorkflowAction,
}

#[derive(Component)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub is_paused: bool,
    pub desired_worker_count: u32,
}

impl Workflow {
    pub fn next_step(&self, current: usize) -> usize {
        if self.steps.is_empty() {
            return 0;
        }
        (current + 1) % self.steps.len()
    }
}

#[derive(Component)]
pub struct WorkflowAssignment {
    pub workflow: Entity,
    pub current_step: usize,
}

#[derive(Component)]
pub struct WaitingForItems {
    pub timer: Timer,
}

impl Default for WaitingForItems {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

#[derive(Event)]
pub struct CreateWorkflowEvent {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub desired_worker_count: u32,
}

#[derive(Event)]
pub struct DeleteWorkflowEvent {
    pub workflow: Entity,
}

#[derive(Event)]
pub struct PauseWorkflowEvent {
    pub workflow: Entity,
}

#[derive(Event)]
pub struct AssignWorkersEvent {
    pub workflow: Entity,
    pub workers: Vec<Entity>,
}

#[derive(Event)]
pub struct UnassignWorkersEvent {
    pub workers: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct WorkflowRegistry {
    pub workflows: Vec<Entity>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn workflow_step_construction() {
        let step = WorkflowStep {
            target: Entity::PLACEHOLDER,
            action: WorkflowAction::Pickup(None),
        };
        assert_eq!(step.target, Entity::PLACEHOLDER);
        assert!(matches!(step.action, WorkflowAction::Pickup(None)));
    }

    #[test]
    fn workflow_action_pickup_none() {
        let action = WorkflowAction::Pickup(None);
        assert!(matches!(action, WorkflowAction::Pickup(None)));
    }

    #[test]
    fn workflow_action_pickup_some() {
        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 5);
        let action = WorkflowAction::Pickup(Some(items));
        match &action {
            WorkflowAction::Pickup(Some(map)) => {
                assert_eq!(map.get("iron_ore"), Some(&5));
            }
            _ => panic!("expected Pickup(Some)"),
        }
    }

    #[test]
    fn workflow_action_dropoff_none() {
        let action = WorkflowAction::Dropoff(None);
        assert!(matches!(action, WorkflowAction::Dropoff(None)));
    }

    #[test]
    fn workflow_action_dropoff_some() {
        let mut items = HashMap::new();
        items.insert("copper_plate".to_string(), 10);
        let action = WorkflowAction::Dropoff(Some(items));
        match &action {
            WorkflowAction::Dropoff(Some(map)) => {
                assert_eq!(map.get("copper_plate"), Some(&10));
            }
            _ => panic!("expected Dropoff(Some)"),
        }
    }

    #[test]
    fn workflow_defaults_not_paused() {
        let workflow = Workflow {
            name: "test workflow".to_string(),
            steps: vec![],
            is_paused: false,
            desired_worker_count: 1,
        };
        assert!(!workflow.is_paused);
    }

    #[test]
    fn workflow_assignment_starts_at_zero() {
        let assignment = WorkflowAssignment {
            workflow: Entity::PLACEHOLDER,
            current_step: 0,
        };
        assert_eq!(assignment.current_step, 0);
    }

    #[test]
    fn waiting_for_items_timer_repeating() {
        let waiting = WaitingForItems::default();
        assert_eq!(waiting.timer.mode(), TimerMode::Repeating);
        assert!((waiting.timer.duration().as_secs_f32() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn workflow_registry_default_empty() {
        let registry = WorkflowRegistry::default();
        assert!(registry.workflows.is_empty());
    }

    #[test]
    fn next_step_wraps_around() {
        let workflow = Workflow {
            name: "cycle test".to_string(),
            steps: vec![
                WorkflowStep {
                    target: Entity::PLACEHOLDER,
                    action: WorkflowAction::Pickup(None),
                },
                WorkflowStep {
                    target: Entity::PLACEHOLDER,
                    action: WorkflowAction::Dropoff(None),
                },
            ],
            is_paused: false,
            desired_worker_count: 1,
        };

        assert_eq!(workflow.next_step(0), 1);
        assert_eq!(workflow.next_step(1), 0);
    }

    #[test]
    fn next_step_empty_workflow() {
        let workflow = Workflow {
            name: "empty".to_string(),
            steps: vec![],
            is_paused: false,
            desired_worker_count: 0,
        };
        assert_eq!(workflow.next_step(0), 0);
    }

    #[test]
    fn workflow_action_clone() {
        let mut items = HashMap::new();
        items.insert("coal".to_string(), 3);
        let original = WorkflowAction::Pickup(Some(items));
        let cloned = original.clone();
        match (&original, &cloned) {
            (WorkflowAction::Pickup(Some(a)), WorkflowAction::Pickup(Some(b))) => {
                assert_eq!(a, b);
            }
            _ => panic!("clone did not preserve variant"),
        }
    }

    #[test]
    fn workflow_step_clone() {
        let step = WorkflowStep {
            target: Entity::PLACEHOLDER,
            action: WorkflowAction::Dropoff(None),
        };
        let cloned = step.clone();
        assert_eq!(cloned.target, step.target);
        assert!(matches!(cloned.action, WorkflowAction::Dropoff(None)));
    }
}
