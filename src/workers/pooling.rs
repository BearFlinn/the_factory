use crate::{
    grid::{Grid, Position},
    materials::{Cargo, InventoryAccess},
    systems::NetworkConnectivity,
    workers::{
        calculate_path, AssignedSequence, Worker, WorkerDispatcher, WorkerPath,
        WorkerStateComputation,
    },
};
use bevy::prelude::*;

/// Marker component for workers returning to the hub.
#[derive(Component)]
pub struct ReturningToHub;

/// Hub location constant - center of the 3x3 hub at origin.
const HUB_CENTER: (i32, i32) = (0, 0);

/// Distance threshold to consider a worker "at hub".
const HUB_PROXIMITY_THRESHOLD: i32 = 2;

/// Checks if a position is within the hub area.
fn is_at_hub(pos: Position) -> bool {
    let distance = (pos.x - HUB_CENTER.0).abs() + (pos.y - HUB_CENTER.1).abs();
    distance <= HUB_PROXIMITY_THRESHOLD
}

/// System that sends idle workers back to the hub.
/// Workers with empty cargo and no assignment will pathfind to the hub.
pub fn return_idle_workers_to_hub(
    mut commands: Commands,
    mut workers: Query<
        (
            Entity,
            &Position,
            &AssignedSequence,
            &Cargo,
            &mut WorkerPath,
            Option<&ReturningToHub>,
        ),
        With<Worker>,
    >,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    mut dispatcher: ResMut<WorkerDispatcher>,
) {
    for (worker_entity, worker_pos, assigned_sequence, cargo, mut worker_path, returning) in
        &mut workers
    {
        let is_idle = assigned_sequence.is_idle() && cargo.is_empty();

        if !is_idle {
            // Worker is busy - remove returning marker if present and unpool
            if returning.is_some() {
                commands.entity(worker_entity).remove::<ReturningToHub>();
            }
            dispatcher.unpool_worker(worker_entity);
            continue;
        }

        // Worker is idle with empty cargo
        if is_at_hub(*worker_pos) {
            // Already at hub - add to pool
            if returning.is_some() {
                commands.entity(worker_entity).remove::<ReturningToHub>();
            }
            dispatcher.pool_worker(worker_entity);

            // Clear any remaining path
            if !worker_path.waypoints.is_empty() || worker_path.current_target.is_some() {
                worker_path.waypoints.clear();
                worker_path.current_target = None;
            }
        } else if returning.is_none() {
            // Not at hub and not already returning - start returning
            if initiate_return_to_hub(*worker_pos, &mut worker_path, &grid, &network) {
                commands.entity(worker_entity).insert(ReturningToHub);
                dispatcher.unpool_worker(worker_entity);
                println!(
                    "Worker {worker_entity:?} at ({}, {}) returning to hub",
                    worker_pos.x, worker_pos.y
                );
            }
        }
        // If returning.is_some() but not at hub, worker is already pathfinding to hub
    }
}

/// Initiates pathfinding to the hub center.
fn initiate_return_to_hub(
    worker_pos: Position,
    worker_path: &mut WorkerPath,
    grid: &Grid,
    network: &NetworkConnectivity,
) -> bool {
    // Already have a path or target
    if !worker_path.waypoints.is_empty() || worker_path.current_target.is_some() {
        return false;
    }

    let start = (worker_pos.x, worker_pos.y);

    // If not connected to network, can't pathfind
    if !network.is_cell_connected(start.0, start.1) {
        return false;
    }

    // Find the closest hub cell that is connected
    if let Some(path) = calculate_path(start, HUB_CENTER, network, grid) {
        worker_path.waypoints = path;
        worker_path.current_target = worker_path.waypoints.pop_front();
        return true;
    }

    // If direct path fails, try adjacent hub cells
    for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
        let target = (HUB_CENTER.0 + dx, HUB_CENTER.1 + dy);
        if let Some(path) = calculate_path(start, target, network, grid) {
            worker_path.waypoints = path;
            worker_path.current_target = worker_path.waypoints.pop_front();
            return true;
        }
    }

    false
}

/// System that marks workers as pooled when they arrive at the hub.
pub fn register_hub_arrivals(
    mut commands: Commands,
    workers: Query<
        (Entity, &Position, &AssignedSequence, &Cargo, &WorkerPath),
        (With<Worker>, With<ReturningToHub>),
    >,
    mut dispatcher: ResMut<WorkerDispatcher>,
) {
    for (worker_entity, worker_pos, assigned_sequence, cargo, worker_path) in &workers {
        // Check if arrived at hub (path complete and at hub location)
        let path_complete =
            worker_path.waypoints.is_empty() && worker_path.current_target.is_none();

        if path_complete
            && is_at_hub(*worker_pos)
            && assigned_sequence.is_idle()
            && cargo.is_empty()
        {
            commands.entity(worker_entity).remove::<ReturningToHub>();
            dispatcher.pool_worker(worker_entity);
            println!(
                "Worker {worker_entity:?} arrived at hub, now pooled (pos: {}, {})",
                worker_pos.x, worker_pos.y
            );
        }
    }
}

/// Clears the `ReturningToHub` marker if worker gets assigned a task.
pub fn clear_returning_on_assignment(
    mut commands: Commands,
    workers: Query<(Entity, &AssignedSequence), (With<Worker>, With<ReturningToHub>)>,
    mut dispatcher: ResMut<WorkerDispatcher>,
) {
    for (worker_entity, assigned_sequence) in workers.iter() {
        if assigned_sequence.is_working() {
            commands.entity(worker_entity).remove::<ReturningToHub>();
            dispatcher.unpool_worker(worker_entity);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn is_at_hub_returns_true_for_center() {
        let pos = Position { x: 0, y: 0 };
        assert!(is_at_hub(pos));
    }

    #[test]
    fn is_at_hub_returns_true_for_adjacent() {
        let positions = [
            Position { x: 1, y: 0 },
            Position { x: -1, y: 0 },
            Position { x: 0, y: 1 },
            Position { x: 0, y: -1 },
            Position { x: 1, y: 1 },
        ];

        for pos in positions {
            assert!(
                is_at_hub(pos),
                "Position ({}, {}) should be at hub",
                pos.x,
                pos.y
            );
        }
    }

    #[test]
    fn is_at_hub_returns_false_for_distant() {
        let positions = [
            Position { x: 5, y: 0 },
            Position { x: 0, y: 5 },
            Position { x: 3, y: 3 },
            Position { x: -5, y: -5 },
        ];

        for pos in positions {
            assert!(
                !is_at_hub(pos),
                "Position ({}, {}) should not be at hub",
                pos.x,
                pos.y
            );
        }
    }

    #[test]
    fn is_at_hub_boundary_check() {
        // Distance 2 should be at hub
        let at_hub = Position { x: 2, y: 0 };
        assert!(is_at_hub(at_hub));

        // Distance 3 should not be at hub
        let not_at_hub = Position { x: 3, y: 0 };
        assert!(!is_at_hub(not_at_hub));
    }
}
