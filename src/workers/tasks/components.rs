use crate::grid::Position;
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Component)]
pub struct Task;

#[derive(Component, PartialEq, Eq, Hash, Clone, Debug)]
#[allow(dead_code)] // TODO: Implement priority
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Component, PartialEq, Debug)]
pub enum TaskStatus {
    Pending,
    Queued,
    InProgress,
    Completed,
}

#[derive(Component, Clone, Debug)]
pub enum TaskAction {
    Pickup(Option<std::collections::HashMap<crate::materials::ItemName, u32>>),
    Dropoff(Option<std::collections::HashMap<crate::materials::ItemName, u32>>),
}

#[derive(Component)]
pub struct TaskTarget(pub Entity);

#[derive(Component)]
pub struct SequenceMember(pub Entity);

#[derive(Component)]
pub struct AssignedWorker(pub Option<Entity>);

#[derive(Bundle)]
pub struct TaskBundle {
    task: Task,
    priority: Priority,
    position: Position,
    task_status: TaskStatus,
    task_target: TaskTarget,
    task_action: TaskAction,
    assigned_worker: AssignedWorker,
}

impl TaskBundle {
    pub fn new(
        task_target: Entity,
        position: Position,
        action: TaskAction,
        priority: Priority,
    ) -> Self {
        Self {
            task: Task,
            priority,
            position,
            task_status: TaskStatus::Pending,
            task_target: TaskTarget(task_target),
            task_action: action,
            assigned_worker: AssignedWorker(None),
        }
    }
}

#[derive(Component)]
pub struct TaskSequence {
    pub tasks: VecDeque<Entity>,
    pub current_index: usize,
}

impl TaskSequence {
    pub fn new(tasks: Vec<Entity>) -> Self {
        Self {
            tasks: VecDeque::from(tasks),
            current_index: 0,
        }
    }

    pub fn current_task(&self) -> Option<Entity> {
        self.tasks.get(self.current_index).copied()
    }

    pub fn advance_to_next(&mut self) -> Option<Entity> {
        self.current_index += 1;
        self.current_task()
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.tasks.len()
    }

    pub fn remaining_tasks(&self) -> usize {
        self.tasks.len().saturating_sub(self.current_index)
    }

    pub fn is_complete_with_validation(&self, task_query: &Query<Entity, With<Task>>) -> bool {
        if self.current_index >= self.tasks.len() {
            return true;
        }

        if let Some(current_task) = self.current_task() {
            if task_query.get(current_task).is_err() {
                return true;
            }
        }

        false
    }

    pub fn validate_and_advance(&mut self, task_query: &Query<Entity, With<Task>>) -> bool {
        let mut advanced = false;

        while self.current_index < self.tasks.len() {
            if let Some(current_task) = self.current_task() {
                if task_query.get(current_task).is_ok() {
                    break;
                }
            }

            self.current_index += 1;
            advanced = true;
        }

        advanced
    }
}

#[derive(Bundle)]
pub struct TaskSequenceBundle {
    pub sequence: TaskSequence,
    pub priority: Priority,
    pub assigned_worker: AssignedWorker,
}

impl TaskSequenceBundle {
    pub fn new(tasks: Vec<Entity>, priority: Priority) -> Self {
        Self {
            sequence: TaskSequence::new(tasks),
            priority,
            assigned_worker: AssignedWorker(None),
        }
    }
}

#[derive(Event)]
pub struct WorkerInterruptEvent {
    pub worker: Entity,
    pub interrupt_type: InterruptType,
}

#[derive(Component)]
pub struct PendingEmergencyDropoff;

#[derive(Debug)]
pub enum InterruptType {
    /// Replace current sequence with an existing sequence entity
    ReplaceSequence(Entity),
    /// Replace current assignment with new tasks (creates new sequence)
    ReplaceTasks(Vec<Entity>, Priority),
    /// Clear current assignment entirely
    ClearAssignment,
}

#[derive(Event)]
pub struct LogisticsDeliveryStartedEvent {
    pub building: Entity,
    pub items: std::collections::HashMap<crate::materials::ItemName, u32>,
}

#[derive(Event)]
pub struct LogisticsDeliveryCompletedEvent {
    pub building: Entity,
    pub items: std::collections::HashMap<crate::materials::ItemName, u32>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    // Helper function to create mock entities for testing
    fn mock_entities(count: usize) -> Vec<Entity> {
        (0..count).map(|i| Entity::from_raw(i as u32)).collect()
    }

    // TaskSequence::new() tests
    #[test]
    fn task_sequence_new_creates_correct_initial_state() {
        let entities = mock_entities(3);
        let sequence = TaskSequence::new(entities.clone());

        assert_eq!(sequence.tasks.len(), 3);
        assert_eq!(sequence.current_index, 0);
        assert_eq!(sequence.tasks[0], entities[0]);
        assert_eq!(sequence.tasks[1], entities[1]);
        assert_eq!(sequence.tasks[2], entities[2]);
    }

    #[test]
    fn task_sequence_new_with_empty_tasks() {
        let sequence = TaskSequence::new(vec![]);

        assert_eq!(sequence.tasks.len(), 0);
        assert_eq!(sequence.current_index, 0);
    }

    // TaskSequence::current_task() tests
    #[test]
    fn current_task_returns_first_task_initially() {
        let entities = mock_entities(3);
        let sequence = TaskSequence::new(entities.clone());

        assert_eq!(sequence.current_task(), Some(entities[0]));
    }

    #[test]
    fn current_task_returns_correct_task_after_advance() {
        let entities = mock_entities(3);
        let mut sequence = TaskSequence::new(entities.clone());

        sequence.advance_to_next();
        assert_eq!(sequence.current_task(), Some(entities[1]));

        sequence.advance_to_next();
        assert_eq!(sequence.current_task(), Some(entities[2]));
    }

    #[test]
    fn current_task_returns_none_when_complete() {
        let entities = mock_entities(2);
        let mut sequence = TaskSequence::new(entities);

        sequence.advance_to_next(); // Move to second task
        sequence.advance_to_next(); // Move past the end

        assert_eq!(sequence.current_task(), None);
    }

    #[test]
    fn current_task_returns_none_for_empty_sequence() {
        let sequence = TaskSequence::new(vec![]);

        assert_eq!(sequence.current_task(), None);
    }

    // TaskSequence::advance_to_next() tests
    #[test]
    fn advance_to_next_moves_to_next_task() {
        let entities = mock_entities(3);
        let mut sequence = TaskSequence::new(entities.clone());

        let next = sequence.advance_to_next();
        assert_eq!(next, Some(entities[1]));
        assert_eq!(sequence.current_index, 1);
    }

    #[test]
    fn advance_to_next_handles_reaching_end() {
        let entities = mock_entities(1);
        let mut sequence = TaskSequence::new(entities);

        let next = sequence.advance_to_next();
        assert_eq!(next, None);
        assert_eq!(sequence.current_index, 1);
    }

    #[test]
    fn advance_to_next_beyond_end_returns_none() {
        let entities = mock_entities(2);
        let mut sequence = TaskSequence::new(entities);

        sequence.advance_to_next(); // index = 1
        sequence.advance_to_next(); // index = 2
        let next = sequence.advance_to_next(); // index = 3

        assert_eq!(next, None);
        assert_eq!(sequence.current_index, 3);
    }

    // TaskSequence::is_complete() tests
    #[test]
    fn is_complete_returns_false_initially() {
        let entities = mock_entities(3);
        let sequence = TaskSequence::new(entities);

        assert!(!sequence.is_complete());
    }

    #[test]
    fn is_complete_returns_true_after_all_tasks() {
        let entities = mock_entities(2);
        let mut sequence = TaskSequence::new(entities);

        sequence.advance_to_next(); // index = 1
        assert!(!sequence.is_complete());

        sequence.advance_to_next(); // index = 2
        assert!(sequence.is_complete());
    }

    #[test]
    fn is_complete_returns_true_for_empty_sequence() {
        let sequence = TaskSequence::new(vec![]);

        assert!(sequence.is_complete());
    }

    // TaskSequence::remaining_tasks() tests
    #[test]
    fn remaining_tasks_at_start() {
        let entities = mock_entities(5);
        let sequence = TaskSequence::new(entities);

        assert_eq!(sequence.remaining_tasks(), 5);
    }

    #[test]
    fn remaining_tasks_in_middle() {
        let entities = mock_entities(5);
        let mut sequence = TaskSequence::new(entities);

        sequence.advance_to_next();
        sequence.advance_to_next();

        assert_eq!(sequence.remaining_tasks(), 3);
    }

    #[test]
    fn remaining_tasks_at_end() {
        let entities = mock_entities(3);
        let mut sequence = TaskSequence::new(entities);

        sequence.advance_to_next();
        sequence.advance_to_next();
        sequence.advance_to_next();

        assert_eq!(sequence.remaining_tasks(), 0);
    }

    #[test]
    fn remaining_tasks_for_empty_sequence() {
        let sequence = TaskSequence::new(vec![]);

        assert_eq!(sequence.remaining_tasks(), 0);
    }

    // TaskBundle::new() tests
    #[test]
    fn task_bundle_new_creates_correct_bundle() {
        let target_entity = Entity::from_raw(42);
        let position = Position { x: 10, y: 20 };
        let action = TaskAction::Pickup(None);

        let bundle = TaskBundle::new(target_entity, position, action.clone(), Priority::High);

        assert_eq!(bundle.task_target.0, target_entity);
        assert_eq!(bundle.position.x, 10);
        assert_eq!(bundle.position.y, 20);
        assert!(bundle.task_status == TaskStatus::Pending);
        assert_eq!(bundle.priority, Priority::High);
        assert!(bundle.assigned_worker.0.is_none());
    }

    #[test]
    fn task_bundle_new_with_specific_items() {
        let target_entity = Entity::from_raw(1);
        let position = Position { x: 5, y: 5 };
        let mut items = std::collections::HashMap::new();
        items.insert("iron".to_string(), 10);
        let action = TaskAction::Dropoff(Some(items));

        let bundle = TaskBundle::new(target_entity, position, action, Priority::Critical);

        assert_eq!(bundle.priority, Priority::Critical);
        if let TaskAction::Dropoff(Some(items)) = bundle.task_action {
            assert_eq!(items.get("iron"), Some(&10));
        } else {
            panic!("Expected Dropoff action with items");
        }
    }

    // TaskSequenceBundle::new() tests
    #[test]
    fn task_sequence_bundle_new_creates_correct_bundle() {
        let entities = mock_entities(3);
        let bundle = TaskSequenceBundle::new(entities.clone(), Priority::Medium);

        assert_eq!(bundle.sequence.tasks.len(), 3);
        assert_eq!(bundle.sequence.current_index, 0);
        assert_eq!(bundle.priority, Priority::Medium);
        assert!(bundle.assigned_worker.0.is_none());
    }

    #[test]
    fn task_sequence_bundle_new_with_empty_tasks() {
        let bundle = TaskSequenceBundle::new(vec![], Priority::Low);

        assert_eq!(bundle.sequence.tasks.len(), 0);
        assert!(bundle.sequence.is_complete());
        assert_eq!(bundle.priority, Priority::Low);
    }

    // Priority equality tests
    #[test]
    fn priority_enum_equality() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_eq!(Priority::Medium, Priority::Medium);
        assert_eq!(Priority::High, Priority::High);
        assert_eq!(Priority::Critical, Priority::Critical);
        assert_ne!(Priority::Low, Priority::High);
    }

    // TaskStatus equality tests
    #[test]
    fn task_status_enum_equality() {
        assert!(TaskStatus::Pending == TaskStatus::Pending);
        assert!(TaskStatus::Queued == TaskStatus::Queued);
        assert!(TaskStatus::InProgress == TaskStatus::InProgress);
        assert!(TaskStatus::Completed == TaskStatus::Completed);
        assert!(TaskStatus::Pending != TaskStatus::Completed);
    }
}
