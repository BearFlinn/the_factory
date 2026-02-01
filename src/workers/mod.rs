pub mod pathfinding;
pub mod spawning;
pub mod workflows;

pub use pathfinding::*;
pub use spawning::*;
pub use workflows::*;

use bevy::prelude::*;

use crate::structures::BuildingSystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorkersSystemSet {
    Lifecycle,
    TaskManagement,
    Movement,
    Interaction,
}

pub struct WorkersPlugin;

impl Plugin for WorkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WorkerArrivedEvent>()
            .add_plugins(WorkflowsPlugin)
            .configure_sets(
                Update,
                (
                    WorkersSystemSet::Lifecycle,
                    WorkersSystemSet::TaskManagement,
                    WorkersSystemSet::Movement,
                    WorkersSystemSet::Interaction,
                )
                    .chain()
                    .in_set(crate::GameplaySet::DomainOperations)
                    .after(BuildingSystemSet::Placement),
            )
            .add_systems(
                Update,
                (
                    validate_and_displace_stranded_workers.in_set(WorkersSystemSet::Lifecycle),
                    move_workers.in_set(WorkersSystemSet::Movement),
                ),
            );
    }
}
