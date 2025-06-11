pub mod spawning;
pub mod pathfinding;
pub mod tasks;

pub use spawning::*;
pub use pathfinding::*;
pub use tasks::*;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum WorkersSystemSet {
    Lifecycle,     // spawning/despawning
    TaskManagement, // task assignment and processing
    Movement,      // pathfinding and movement
    Interaction,   // arrivals and transfers
}

pub struct WorkersPlugin;

impl Plugin for WorkersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<WorkerArrivedEvent>()
            .add_plugins(TasksPlugin)
            .configure_sets(Update, (
                WorkersSystemSet::Lifecycle,     // spawning/despawning
                WorkersSystemSet::TaskManagement, // task assignment and processing  
                WorkersSystemSet::Movement,      // pathfinding and movement
                WorkersSystemSet::Interaction,   // arrivals and transfers
            ).chain().in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (
                // Displacement system to run early
                validate_and_displace_stranded_workers
                    .in_set(WorkersSystemSet::Lifecycle),
                    
                // Movement - unchanged
                move_workers
                    .in_set(WorkersSystemSet::Movement),
            ));
    }
}