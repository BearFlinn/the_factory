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
            .configure_sets(Update, (
                WorkersSystemSet::Lifecycle,
                WorkersSystemSet::TaskManagement,
                WorkersSystemSet::Movement,
                WorkersSystemSet::Interaction,
            ).chain().in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (
                // Task management
                (assign_available_workers_to_tasks, assign_worker_to_tasks, process_worker_tasks, create_logistics_tasks, clear_all_tasks)
                    .in_set(WorkersSystemSet::TaskManagement),
                // Movement
                move_workers
                    .in_set(WorkersSystemSet::Movement),
                    
                // Arrivals and cleanup
                (handle_task_arrivals, clear_completed_tasks)
                    .in_set(WorkersSystemSet::Interaction),
            ));
    }
}
