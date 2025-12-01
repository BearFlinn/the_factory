pub mod spawning;
pub mod pathfinding;
pub mod tasks;

pub use spawning::*;
pub use pathfinding::*;
pub use tasks::*;

use bevy::prelude::*;

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
        app
            .add_event::<WorkerArrivedEvent>()
            .add_plugins(TasksPlugin)
            .configure_sets(Update, (
                WorkersSystemSet::Lifecycle, 
                WorkersSystemSet::TaskManagement, 
                WorkersSystemSet::Movement, 
                WorkersSystemSet::Interaction
            ).chain()
                .in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (  
                validate_and_displace_stranded_workers,
                move_workers
                    .in_set(WorkersSystemSet::Movement),
            ));
    }
}
