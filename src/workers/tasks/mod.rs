pub mod components;
pub mod assignment;
pub mod creation;
pub mod execution;

// Re-export all public items
pub use components::*;
pub use assignment::*;
pub use creation::*;
pub use execution::*;

use bevy::prelude::*;
use crate::workers::WorkersSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum TaskSystemSet {
    Interrupts,      // Handle worker interruptions and state resets
    Assignment,      // Assign sequences to available workers  
    Processing,      // Process worker sequences and update states
    Generation,      // Generate new task sequences from requests
    Cleanup,         // Clean up completed tasks and sequences
}

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<WorkerInterruptEvent>()
            .configure_sets(Update, (
                TaskSystemSet::Interrupts,
                TaskSystemSet::Assignment, 
                TaskSystemSet::Processing,
                TaskSystemSet::Generation,
                TaskSystemSet::Cleanup,
            ).chain().in_set(WorkersSystemSet::TaskManagement))
            .add_systems(Update, (
                // Interrupt handling - highest priority
                (handle_worker_interrupts, debug_clear_all_workers, emergency_dropoff_idle_workers)
                    .in_set(TaskSystemSet::Interrupts),
                
                // Worker assignment
                assign_available_sequences_to_workers
                    .in_set(TaskSystemSet::Assignment),
                
                // Sequence processing and state management
                (process_worker_sequences, derive_worker_state_from_sequences)
                    .chain()
                    .in_set(TaskSystemSet::Processing),
                
                // Task generation from external requests - UPDATED to include construction logistics
                (create_logistics_tasks, create_construction_logistics_tasks, clear_all_tasks)
                    .in_set(TaskSystemSet::Generation),
                
                // Cleanup systems
                (handle_sequence_task_arrivals, clear_completed_tasks)
                    .in_set(TaskSystemSet::Cleanup),
            ));
    }
}