use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    systems::NetworkConnectivity, workers::{Speed, Worker}
};

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
    mut workers: Query<(Entity, &mut Transform, &mut WorkerPath, &mut Position, &Speed), With<Worker>>,
    grid: Res<Grid>,
    time: Res<Time>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    for (worker_entity, mut transform, mut path, mut worker_pos, speed) in workers.iter_mut() {
        // Move toward current target
        if let Some(target) = path.current_target {
            let direction = (target - transform.translation.truncate()).normalize_or_zero();
            let movement = direction * speed.value * time.delta_secs();
            transform.translation += movement.extend(0.0);
            
            let distance_to_target = (target - transform.translation.truncate()).length();
            
            if distance_to_target <= 1.0 {
                let target_coords = grid.world_to_grid_coordinates(target).unwrap();
                *worker_pos = Position { x: target_coords.grid_x, y: target_coords.grid_y };
                path.current_target = path.waypoints.pop_front();
                
                if path.current_target.is_none() {
                    if let Some(target_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                        arrival_events.send(WorkerArrivedEvent {
                            worker: worker_entity,
                            position: (target_coords.grid_x, target_coords.grid_y),
                        });
                    }
                    continue;
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
    
    // Validate that start and end are in the extended network (connected to infrastructure)
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
            // Reconstruct path
            let mut path = Vec::new();
            let mut current_pos = end;
            
            while current_pos != start {
                path.push(current_pos);
                current_pos = parent[&current_pos];
            }
            
            path.reverse();
            
            // Convert to world coordinates
            let world_path = path.into_iter()
                .map(|(x, y)| grid.grid_to_world_coordinates(x, y))
                .collect();
            
            return Some(world_path);
        }
        
        // Check adjacent cells
        for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let next = (current.0 + dx, current.1 + dy);
            
            if visited.contains(&next) {
                continue;
            }
            
            // Allow movement to core network cells, or to the end destination if it's in extended network
            let can_move_to_cell = network.is_core_network_cell(next.0, next.1) || 
                                  (next == end && network.is_cell_connected(next.0, next.1));
            
            if can_move_to_cell {
                visited.insert(next);
                parent.insert(next, current);
                queue.push_back(next);
            }
        }
    }
    println!("No path found");
    None // No path found
}