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
                    (handle_worker_interrupts, debug_clear_all_workers)
                        .in_set(TaskSystemSet::Interrupts),
                    emergency_dropoff_idle_workers
                        .in_set(TaskSystemSet::Interrupts)
                        .after(execute_item_transfer)
                        .after(handle_worker_interrupts),
                    assign_available_sequences_to_workers.in_set(TaskSystemSet::Assignment),
                    (process_worker_sequences, derive_worker_state_from_sequences)
                        .chain()
                        .in_set(TaskSystemSet::Processing),
                    (
                        create_port_logistics_tasks,
                        create_proactive_port_tasks,
                        create_port_construction_logistics_tasks,
                        clear_all_tasks,
                    )
                        .in_set(TaskSystemSet::Generation),
                    (handle_sequence_task_arrivals, clear_completed_tasks)
                        .in_set(TaskSystemSet::Cleanup),
                ),
            );
    }
}
