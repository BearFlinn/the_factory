// workers/mod.rs
pub mod spawning;
pub mod pathfinding;

pub use spawning::*;
pub use pathfinding::*;

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
            .configure_sets(Update, (
                WorkersSystemSet::Lifecycle,
                WorkersSystemSet::Movement,
                WorkersSystemSet::Interaction,
            ).chain().in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (

                move_workers
                    .in_set(WorkersSystemSet::Movement),
                
                handle_worker_arrivals
                    .in_set(WorkersSystemSet::Interaction),
            ));
    }
}