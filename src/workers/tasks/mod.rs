pub mod components;
pub mod assignment;
pub mod logistics;
pub mod execution;

// Re-export all public items
pub use components::*;
pub use assignment::*;
pub use logistics::*;
pub use execution::*;

use bevy::prelude::*;
use crate::workers::WorkersSystemSet;

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            // Task management systems
            (assign_available_sequences_to_workers, process_worker_sequences, derive_worker_state_from_sequences, create_logistics_tasks, clear_all_tasks)
                .in_set(WorkersSystemSet::TaskManagement),
                
            // Task completion and cleanup systems
            (handle_sequence_task_arrivals, clear_completed_tasks)
                .in_set(WorkersSystemSet::Interaction),
        ));
    }
}