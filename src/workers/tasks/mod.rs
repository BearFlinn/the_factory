pub mod assignment;
pub mod components;
pub mod creation;
pub mod execution;

pub use assignment::*;
pub use components::*;
pub use creation::*;
pub use execution::*;

use crate::{
    materials::execute_item_transfer, structures::RecipeCommitment, workers::WorkersSystemSet,
};
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum TaskSystemSet {
    Interrupts,
    Assignment,
    Processing,
    Arrivals,
    Generation,
    Cleanup,
}

pub struct TasksPlugin;

fn update_in_transit_tracking(
    mut started_events: EventReader<LogisticsDeliveryStartedEvent>,
    mut completed_events: EventReader<LogisticsDeliveryCompletedEvent>,
    mut commitments: Query<&mut RecipeCommitment>,
) {
    for event in started_events.read() {
        if let Ok(mut commitment) = commitments.get_mut(event.building) {
            commitment.add_in_transit(&event.items);
        }
    }
    for event in completed_events.read() {
        if let Ok(mut commitment) = commitments.get_mut(event.building) {
            commitment.remove_in_transit(&event.items);
        }
    }
}

impl Plugin for TasksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WorkerInterruptEvent>()
            .add_event::<LogisticsDeliveryStartedEvent>()
            .add_event::<LogisticsDeliveryCompletedEvent>()
            .init_resource::<ProactiveTaskTimer>()
            .configure_sets(
                Update,
                (
                    TaskSystemSet::Interrupts,
                    TaskSystemSet::Assignment,
                    TaskSystemSet::Processing,
                    TaskSystemSet::Arrivals,
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
                    process_worker_sequences.in_set(TaskSystemSet::Processing),
                    handle_sequence_task_arrivals.in_set(TaskSystemSet::Arrivals),
                    (
                        create_port_logistics_tasks,
                        create_proactive_port_tasks,
                        create_port_construction_logistics_tasks,
                        clear_all_tasks,
                    )
                        .in_set(TaskSystemSet::Generation),
                    (
                        clear_completed_tasks,
                        update_in_transit_tracking,
                        cleanup_emergency_dropoff_markers,
                    )
                        .in_set(TaskSystemSet::Cleanup),
                ),
            );
    }
}
