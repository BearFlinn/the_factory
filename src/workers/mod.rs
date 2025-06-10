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
                WorkersSystemSet::Lifecycle,     // spawning/despawning
                WorkersSystemSet::TaskManagement, // task assignment and processing  
                WorkersSystemSet::Movement,      // pathfinding and movement
                WorkersSystemSet::Interaction,   // arrivals and transfers
            ).chain().in_set(crate::GameplaySet::DomainOperations))
            .add_systems(Update, (
                // NEW: Add displacement system to Lifecycle set to run early
                validate_and_displace_stranded_workers
                    .in_set(WorkersSystemSet::Lifecycle),
                    
                // Task management - updated for defensive sequence architecture
                (assign_available_sequences_to_workers, process_worker_sequences_defensive, derive_worker_state_from_sequences, create_logistics_tasks, clear_all_tasks)
                    .in_set(WorkersSystemSet::TaskManagement),
                    
                // Movement - unchanged
                move_workers
                    .in_set(WorkersSystemSet::Movement),
                    
                // Arrivals and cleanup - updated for defensive sequence architecture
                (handle_sequence_task_arrivals_defensive, clear_completed_tasks)
                    .in_set(WorkersSystemSet::Interaction),
            ));
    }
}
