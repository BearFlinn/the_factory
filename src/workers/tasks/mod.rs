pub mod assignment;
pub mod components;
pub mod creation;
pub mod execution;

pub use assignment::*;
pub use components::*;
pub use creation::*;
pub use execution::*;

use crate::{materials::execute_item_transfer, workers::WorkersSystemSet};
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum TaskSystemSet {
    Interrupts,
    Assignment,
    Processing,
    Generation,
    Cleanup,
}

pub struct TasksPlugin;

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WorkerInterruptEvent>()
            .init_resource::<ProactiveTaskTimer>()
            .configure_sets(
                Update,
                (
                    TaskSystemSet::Interrupts,
                    TaskSystemSet::Assignment,
                    TaskSystemSet::Processing,
                    TaskSystemSet::Generation,
                    TaskSystemSet::Cleanup,
                )
                    .chain()
                    .in_set(WorkersSystemSet::TaskManagement),
            )
            .add_systems(
                Update,
                (
                    // Interrupt handling - highest priority
                    (handle_worker_interrupts, debug_clear_all_workers)
                        .in_set(TaskSystemSet::Interrupts),
                    // Emergency dropoff must run after item transfers complete to avoid
                    // creating redundant tasks for workers who just completed a dropoff
                    emergency_dropoff_idle_workers
                        .in_set(TaskSystemSet::Interrupts)
                        .after(execute_item_transfer),
                    // Worker assignment
                    assign_available_sequences_to_workers.in_set(TaskSystemSet::Assignment),
                    // Sequence processing and state management
                    (process_worker_sequences, derive_worker_state_from_sequences)
                        .chain()
                        .in_set(TaskSystemSet::Processing),
                    // Port-based logistics systems
                    (
                        create_port_logistics_tasks,
                        create_proactive_port_tasks,
                        create_port_construction_logistics_tasks,
                        clear_all_tasks,
                    )
                        .in_set(TaskSystemSet::Generation),
                    // Cleanup systems
                    (handle_sequence_task_arrivals, clear_completed_tasks)
                        .in_set(TaskSystemSet::Cleanup),
                ),
            );
    }
}
