use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::materials::ItemName;

#[derive(Clone, Debug)]
pub enum WorkflowAction {
    Pickup(Option<HashMap<ItemName, u32>>),
    Dropoff(Option<HashMap<ItemName, u32>>),
}

#[derive(Clone, Debug)]
pub enum StepTarget {
    Specific(Entity),
    ByType(String),
}

#[derive(Clone, Debug)]
pub struct WorkflowStep {
    pub target: StepTarget,
    pub action: WorkflowAction,
}

#[derive(Component)]
pub struct Workflow {
    pub name: String,
    pub building_set: HashSet<Entity>,
    pub steps: Vec<WorkflowStep>,
    pub is_paused: bool,
    pub desired_worker_count: u32,
    pub round_robin_counters: HashMap<usize, usize>,
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
    pub resolved_target: Option<Entity>,
    pub resolved_action: Option<WorkflowAction>,
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

#[derive(Message)]
pub struct CreateWorkflowEvent {
    pub name: String,
    pub building_set: HashSet<Entity>,
    pub steps: Vec<WorkflowStep>,
    pub desired_worker_count: u32,
}

#[derive(Message)]
pub struct UpdateWorkflowEvent {
    pub entity: Entity,
    pub name: String,
    pub building_set: HashSet<Entity>,
    pub steps: Vec<WorkflowStep>,
    pub desired_worker_count: u32,
}

#[derive(Message)]
pub struct DeleteWorkflowEvent {
    pub workflow: Entity,
}

#[derive(Message)]
pub struct PauseWorkflowEvent {
    pub workflow: Entity,
}

#[derive(Message)]
pub struct AssignWorkersEvent {
    pub workflow: Entity,
    pub workers: Vec<Entity>,
}

#[derive(Message)]
pub struct UnassignWorkersEvent {
    pub workers: Vec<Entity>,
}

#[derive(Message)]
pub struct BatchAssignWorkersEvent {
    pub workflow: Entity,
    pub count: u32,
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
    fn workflow_step_construction_specific() {
        let step = WorkflowStep {
            target: StepTarget::Specific(Entity::PLACEHOLDER),
            action: WorkflowAction::Pickup(None),
        };
        assert!(matches!(step.target, StepTarget::Specific(_)));
        assert!(matches!(step.action, WorkflowAction::Pickup(None)));
    }

    #[test]
    fn workflow_step_construction_by_type() {
        let step = WorkflowStep {
            target: StepTarget::ByType("Smelter".to_string()),
            action: WorkflowAction::Dropoff(None),
        };
        match &step.target {
            StepTarget::ByType(name) => assert_eq!(name, "Smelter"),
            StepTarget::Specific(_) => panic!("expected ByType"),
        }
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
            building_set: HashSet::new(),
            steps: vec![],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        };
        assert!(!workflow.is_paused);
    }

    #[test]
    fn workflow_assignment_starts_at_zero() {
        let assignment = WorkflowAssignment {
            workflow: Entity::PLACEHOLDER,
            current_step: 0,
            resolved_target: None,
            resolved_action: None,
        };
        assert_eq!(assignment.current_step, 0);
        assert!(assignment.resolved_target.is_none());
        assert!(assignment.resolved_action.is_none());
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
            building_set: HashSet::new(),
            steps: vec![
                WorkflowStep {
                    target: StepTarget::Specific(Entity::PLACEHOLDER),
                    action: WorkflowAction::Pickup(None),
                },
                WorkflowStep {
                    target: StepTarget::Specific(Entity::PLACEHOLDER),
                    action: WorkflowAction::Dropoff(None),
                },
            ],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        };

        assert_eq!(workflow.next_step(0), 1);
        assert_eq!(workflow.next_step(1), 0);
    }

    #[test]
    fn next_step_empty_workflow() {
        let workflow = Workflow {
            name: "empty".to_string(),
            building_set: HashSet::new(),
            steps: vec![],
            is_paused: false,
            desired_worker_count: 0,
            round_robin_counters: HashMap::new(),
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
            target: StepTarget::Specific(Entity::PLACEHOLDER),
            action: WorkflowAction::Dropoff(None),
        };
        let cloned = step.clone();
        assert!(matches!(cloned.target, StepTarget::Specific(_)));
        assert!(matches!(cloned.action, WorkflowAction::Dropoff(None)));
    }

    #[test]
    fn building_set_tracks_entities() {
        let mut set = HashSet::new();
        set.insert(Entity::PLACEHOLDER);
        let workflow = Workflow {
            name: "pool test".to_string(),
            building_set: set,
            steps: vec![],
            is_paused: false,
            desired_worker_count: 1,
            round_robin_counters: HashMap::new(),
        };
        assert!(workflow.building_set.contains(&Entity::PLACEHOLDER));
        assert_eq!(workflow.building_set.len(), 1);
    }

    #[test]
    fn step_target_by_type_clone() {
        let target = StepTarget::ByType("Mining Drill".to_string());
        let cloned = target.clone();
        match cloned {
            StepTarget::ByType(name) => assert_eq!(name, "Mining Drill"),
            StepTarget::Specific(_) => panic!("clone did not preserve ByType"),
        }
    }
}
