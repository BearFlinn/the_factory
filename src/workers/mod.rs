pub mod spawning;
pub mod pathfinding;
pub mod orchestrator;

pub use spawning::*;
pub use pathfinding::*;
pub use orchestrator::*;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorkersSystemSet {
    Lifecycle,     // spawning/despawning
    Movement,      // pathfinding and movement
    Interaction,   // arrivals and transfers
}

pub struct WorkersPlugin;

impl Plugin for WorkersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<WorkerArrivedEvent>()
            .add_event::<DesignateTask>()
            .configure_sets(Update, (
                WorkersSystemSet::Lifecycle,
                WorkersSystemSet::Movement,
                WorkersSystemSet::Interaction,
            ).chain().in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (
                worker_orchestration_system
                    .in_set(WorkersSystemSet::Lifecycle),
                (handle_worker_paths, move_workers)
                    .in_set(WorkersSystemSet::Movement),
                handle_worker_task_arrivals
                    .in_set(WorkersSystemSet::Interaction),
            ));
    }
}
