use crate::{
    grid::{Grid, Position},
    systems::NetworkConnectivity,
    workers::{Speed, Worker, WorkflowAssignment},
};
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

#[derive(Component)]
pub struct WorkerPath {
    pub waypoints: VecDeque<Vec2>,
    pub current_target: Option<Vec2>,
}

#[derive(Event)]
pub struct WorkerArrivedEvent {
    pub worker: Entity,
    pub position: (i32, i32),
}

pub fn move_workers(
    mut workers: Query<
        (
            Entity,
            &mut Transform,
            &mut WorkerPath,
            &mut Position,
            &Speed,
        ),
        With<Worker>,
    >,
    grid: Res<Grid>,
    time: Res<Time>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    for (worker_entity, mut transform, mut path, mut worker_pos, speed) in &mut workers {
        if let Some(target) = path.current_target {
            let direction = (target - transform.translation.truncate()).normalize_or_zero();
            let movement = direction * speed.value * time.delta_secs();
            transform.translation += movement.extend(0.0);

            let distance_to_target = (target - transform.translation.truncate()).length();

            if distance_to_target <= 1.0 {
                let Some(target_coords) = grid.world_to_grid_coordinates(target) else {
                    continue;
                };
                *worker_pos = Position {
                    x: target_coords.grid_x,
                    y: target_coords.grid_y,
                };
                path.current_target = path.waypoints.pop_front();

                if path.current_target.is_none() {
                    if let Some(target_coords) =
                        grid.world_to_grid_coordinates(transform.translation.truncate())
                    {
                        arrival_events.write(WorkerArrivedEvent {
                            worker: worker_entity,
                            position: (target_coords.grid_x, target_coords.grid_y),
                        });
                    }
                }
            }
        }
    }
}

pub fn manhattan_distance_coords(pos1: (i32, i32), pos2: (i32, i32)) -> i32 {
    (pos1.0 - pos2.0).abs() + (pos1.1 - pos2.1).abs()
}

pub fn calculate_path(
    start: (i32, i32),
    end: (i32, i32),
    network: &NetworkConnectivity,
    grid: &Grid,
) -> Option<VecDeque<Vec2>> {
    use std::collections::HashMap;

    if start == end {
        return Some(VecDeque::new());
    }

    if !network.is_cell_connected(start.0, start.1) || !network.is_cell_connected(end.0, end.1) {
        return None;
    }

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent = HashMap::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        if current == end {
            let mut path = Vec::new();
            let mut current_pos = end;

            while current_pos != start {
                path.push(current_pos);
                current_pos = parent[&current_pos];
            }

            path.reverse();

            let world_path = path
                .into_iter()
                .map(|(x, y)| grid.grid_to_world_coordinates(x, y))
                .collect();

            return Some(world_path);
        }

        for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let next = (current.0 + dx, current.1 + dy);

            if visited.contains(&next) {
                continue;
            }

            let can_move_to_cell = network.is_core_network_cell(next.0, next.1)
                || (next == end && network.is_cell_connected(next.0, next.1));

            if can_move_to_cell {
                visited.insert(next);
                parent.insert(next, current);
                queue.push_back(next);
            }
        }
    }
    println!("No path found");
    None
}

pub fn validate_and_displace_stranded_workers(
    mut commands: Commands,
    mut workers: Query<
        (
            Entity,
            &mut Transform,
            &mut Position,
            &mut WorkerPath,
            Option<&WorkflowAssignment>,
        ),
        With<Worker>,
    >,
    network: Res<NetworkConnectivity>,
    grid: Res<Grid>,
) {
    let mut displaced_count = 0;

    for (worker_entity, mut transform, mut worker_position, mut worker_path, has_assignment) in
        &mut workers
    {
        let worker_pos = (worker_position.x, worker_position.y);

        if !network.is_cell_connected(worker_pos.0, worker_pos.1) {
            if let Some(displacement_target) =
                find_nearest_valid_network_cell(worker_pos, &network, 10)
            {
                println!(
                    "Emergency displacement: Worker {worker_entity:?} stranded at {worker_pos:?}, moving to {displacement_target:?}"
                );

                worker_position.x = displacement_target.0;
                worker_position.y = displacement_target.1;

                let world_pos =
                    grid.grid_to_world_coordinates(displacement_target.0, displacement_target.1);
                transform.translation = world_pos.extend(transform.translation.z);

                worker_path.waypoints.clear();
                worker_path.current_target = None;

                if has_assignment.is_some() {
                    commands
                        .entity(worker_entity)
                        .remove::<WorkflowAssignment>();
                }

                displaced_count += 1;
            } else {
                println!(
                    "Critical: Worker {worker_entity:?} stranded at {worker_pos:?} with no reachable valid network cells"
                );

                worker_path.waypoints.clear();
                worker_path.current_target = None;
                if has_assignment.is_some() {
                    commands
                        .entity(worker_entity)
                        .remove::<WorkflowAssignment>();
                }
            }
        }
    }

    if displaced_count > 0 {
        println!("Emergency displacement system relocated {displaced_count} stranded workers");
    }
}

fn find_nearest_valid_network_cell(
    stranded_pos: (i32, i32),
    network: &NetworkConnectivity,
    max_search_radius: i32,
) -> Option<(i32, i32)> {
    for radius in 1..=max_search_radius {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }

                let candidate_pos = (stranded_pos.0 + dx, stranded_pos.1 + dy);

                if network.is_cell_connected(candidate_pos.0, candidate_pos.1) {
                    return Some(candidate_pos);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn manhattan_distance_same_point_returns_zero() {
        let result = manhattan_distance_coords((5, 10), (5, 10));

        assert_eq!(result, 0);
    }

    #[test]
    fn manhattan_distance_horizontal_movement_only() {
        let result = manhattan_distance_coords((0, 0), (7, 0));

        assert_eq!(result, 7);
    }

    #[test]
    fn manhattan_distance_vertical_movement_only() {
        let result = manhattan_distance_coords((0, 0), (0, 12));

        assert_eq!(result, 12);
    }

    #[test]
    fn manhattan_distance_diagonal_movement() {
        let result = manhattan_distance_coords((0, 0), (3, 4));

        assert_eq!(result, 7); // 3 + 4 = 7
    }

    #[test]
    fn manhattan_distance_negative_coordinates() {
        let result = manhattan_distance_coords((-5, -3), (2, 4));

        // |(-5) - 2| + |(-3) - 4| = 7 + 7 = 14
        assert_eq!(result, 14);
    }

    #[test]
    fn manhattan_distance_reversed_direction() {
        // Distance should be the same regardless of direction
        let result1 = manhattan_distance_coords((10, 20), (3, 8));
        let result2 = manhattan_distance_coords((3, 8), (10, 20));

        assert_eq!(result1, result2);
        assert_eq!(result1, 19); // |10-3| + |20-8| = 7 + 12 = 19
    }

    #[test]
    fn manhattan_distance_large_values() {
        let result = manhattan_distance_coords((1000, 2000), (-500, -1000));

        // |1000 - (-500)| + |2000 - (-1000)| = 1500 + 3000 = 4500
        assert_eq!(result, 4500);
    }

    #[test]
    fn manhattan_distance_at_origin() {
        let result = manhattan_distance_coords((0, 0), (0, 0));

        assert_eq!(result, 0);
    }

    #[test]
    fn calculate_path_same_start_and_end_returns_empty_path() {
        let mut network = NetworkConnectivity::default();
        // Need to add the cell to connected_cells for the path calculation
        network.add_connected_cell(5, 5);
        network.add_core_network_cell(5, 5);

        let grid = Grid::new(64.0);

        let result = calculate_path((5, 5), (5, 5), &network, &grid);

        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn calculate_path_returns_none_when_start_not_connected() {
        let mut network = NetworkConnectivity::default();
        // Only end is connected, start is not
        network.add_connected_cell(5, 5);
        network.add_core_network_cell(5, 5);

        let grid = Grid::new(64.0);

        let result = calculate_path((0, 0), (5, 5), &network, &grid);

        assert!(result.is_none());
    }

    #[test]
    fn calculate_path_returns_none_when_end_not_connected() {
        let mut network = NetworkConnectivity::default();
        // Only start is connected, end is not
        network.add_connected_cell(0, 0);
        network.add_core_network_cell(0, 0);

        let grid = Grid::new(64.0);

        let result = calculate_path((0, 0), (5, 5), &network, &grid);

        assert!(result.is_none());
    }

    #[test]
    fn calculate_path_simple_horizontal_path() {
        let mut network = NetworkConnectivity::default();
        // Create a horizontal line of connected cells
        for x in 0..=3 {
            network.add_connected_cell(x, 0);
            network.add_core_network_cell(x, 0);
        }

        let mut grid = Grid::new(64.0);
        for x in 0..=3 {
            grid.add_coordinate(x, 0);
        }

        let result = calculate_path((0, 0), (3, 0), &network, &grid);

        assert!(result.is_some());
        let path = result.unwrap();
        assert_eq!(path.len(), 3); // Should include (1,0), (2,0), (3,0)
    }

    #[test]
    fn calculate_path_simple_vertical_path() {
        let mut network = NetworkConnectivity::default();
        // Create a vertical line of connected cells
        for y in 0..=3 {
            network.add_connected_cell(0, y);
            network.add_core_network_cell(0, y);
        }

        let mut grid = Grid::new(64.0);
        for y in 0..=3 {
            grid.add_coordinate(0, y);
        }

        let result = calculate_path((0, 0), (0, 3), &network, &grid);

        assert!(result.is_some());
        let path = result.unwrap();
        assert_eq!(path.len(), 3); // Should include (0,1), (0,2), (0,3)
    }

    #[test]
    fn calculate_path_with_disconnected_network_returns_none() {
        let mut network = NetworkConnectivity::default();
        // Create two disconnected groups
        network.add_connected_cell(0, 0);
        network.add_core_network_cell(0, 0);

        network.add_connected_cell(5, 5);
        network.add_core_network_cell(5, 5);
        // No path between them

        let grid = Grid::new(64.0);

        let result = calculate_path((0, 0), (5, 5), &network, &grid);

        assert!(result.is_none());
    }

    #[test]
    fn calculate_path_converts_to_world_coordinates() {
        let mut network = NetworkConnectivity::default();
        // Simple 2-cell path
        network.add_connected_cell(0, 0);
        network.add_core_network_cell(0, 0);
        network.add_connected_cell(1, 0);
        network.add_core_network_cell(1, 0);

        let cell_size = 64.0;
        let mut grid = Grid::new(cell_size);
        grid.add_coordinate(0, 0);
        grid.add_coordinate(1, 0);

        let result = calculate_path((0, 0), (1, 0), &network, &grid);

        assert!(result.is_some());
        let path = result.unwrap();
        assert_eq!(path.len(), 1);
        // The path should contain world coordinates for cell (1, 0)
        assert_eq!(path[0], Vec2::new(64.0, 0.0));
    }
}
