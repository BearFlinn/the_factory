use std::collections::VecDeque;
use bevy::prelude::*;
use crate::grid::Position;

#[derive(Component)]
pub struct Task;

#[derive(Component, PartialEq, Eq, Hash, Clone)]
#[allow(dead_code)] // TODO: Implement priority
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Component, PartialEq)]
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
    pub fn new(task_target: Entity, position: Position, action: TaskAction, priority: Priority) -> Self {
        Self {
            task: Task,
            priority: priority,
            position: position,
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
        // First check the basic completion condition
        if self.current_index >= self.tasks.len() {
            return true;
        }
        
        // Check if current task entity still exists
        if let Some(current_task) = self.current_task() {
            if task_query.get(current_task).is_err() {
                return true; // Current task was despawned, consider sequence complete
            }
        }
        
        false
    }
    
    // Add method to validate and clean invalid tasks
    pub fn validate_and_advance(&mut self, task_query: &Query<Entity, With<Task>>) -> bool {
        let mut advanced = false;
        
        // Skip invalid tasks until we find a valid one or reach the end
        while self.current_index < self.tasks.len() {
            if let Some(current_task) = self.current_task() {
                if task_query.get(current_task).is_ok() {
                    break; // Found valid task
                }
            }
            
            // Current task is invalid, advance
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