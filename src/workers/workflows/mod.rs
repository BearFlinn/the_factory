pub mod components;
pub mod execution;
pub mod management;

pub use components::*;
pub use execution::*;
pub use management::*;

use crate::workers::WorkersSystemSet;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorkflowSystemSet {
    Management,
    Processing,
    Arrivals,
    Waiting,
    Cleanup,
}

pub struct WorkflowsPlugin;

impl Plugin for WorkflowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CreateWorkflowEvent>()
            .add_message::<DeleteWorkflowEvent>()
            .add_message::<PauseWorkflowEvent>()
            .add_message::<AssignWorkersEvent>()
            .add_message::<UnassignWorkersEvent>()
            .add_message::<BatchAssignWorkersEvent>()
            .add_message::<UpdateWorkflowEvent>()
            .init_resource::<WorkflowRegistry>()
            .configure_sets(
                Update,
                (
                    WorkflowSystemSet::Management,
                    WorkflowSystemSet::Processing,
                    WorkflowSystemSet::Arrivals,
                    WorkflowSystemSet::Waiting,
                    WorkflowSystemSet::Cleanup,
                )
                    .chain()
                    .in_set(WorkersSystemSet::TaskManagement),
            )
            .add_systems(
                Update,
                (
                    (
                        handle_create_workflow,
                        handle_delete_workflow,
                        handle_pause_workflow,
                        handle_assign_workers,
                        handle_unassign_workers,
                        handle_batch_assign_workers,
                        handle_update_workflow,
                    )
                        .in_set(WorkflowSystemSet::Management),
                    process_workflow_workers.in_set(WorkflowSystemSet::Processing),
                    handle_workflow_arrivals.in_set(WorkflowSystemSet::Arrivals),
                    (recheck_waiting_workers, recheck_waiting_for_space)
                        .in_set(WorkflowSystemSet::Waiting),
                    (
                        cleanup_invalid_workflow_refs,
                        emergency_dropoff_unassigned_workers,
                    )
                        .in_set(WorkflowSystemSet::Cleanup),
                ),
            );
    }
}
