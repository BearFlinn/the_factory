use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    systems::NetworkConnectivity, workers::{AssignedSequence, Speed, Worker, WorkerState}
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
    None
}

pub fn validate_and_displace_stranded_workers(
    mut workers: Query<(Entity, &mut Transform, &mut Position, &mut WorkerPath, &mut WorkerState, &mut AssignedSequence), With<Worker>>,
    network: Res<NetworkConnectivity>,
    grid: Res<Grid>,
) {
    let mut displaced_count = 0;
    
    for (worker_entity, mut transform, mut worker_position, mut worker_path, mut worker_state, mut assigned_sequence) in workers.iter_mut() {
        // Check if worker's current position is still valid in the network
        let worker_pos = (worker_position.x, worker_position.y);
        
        if !network.is_cell_connected(worker_pos.0, worker_pos.1) {
            // Worker is stranded on invalid terrain - find nearest valid position
            if let Some(displacement_target) = find_nearest_valid_network_cell(worker_pos, &network, 10) {
                println!("Emergency displacement: Worker {:?} stranded at {:?}, moving to {:?}", 
                         worker_entity, worker_pos, displacement_target);
                
                // Update worker's grid position
                worker_position.x = displacement_target.0;
                worker_position.y = displacement_target.1;
                
                // Update worker's world transform
                let world_pos = grid.grid_to_world_coordinates(displacement_target.0, displacement_target.1);
                transform.translation = world_pos.extend(transform.translation.z);
                
                // Clear potentially invalid pathfinding state
                worker_path.waypoints.clear();
                worker_path.current_target = None;
                
                // Reset worker state to allow task reassignment
                *worker_state = WorkerState::Idle;
                assigned_sequence.0 = None;
                
                displaced_count += 1;
            } else {
                println!("Critical: Worker {:?} stranded at {:?} with no reachable valid network cells", 
                         worker_entity, worker_pos);
                
                // Fallback: Reset worker state even if we can't move them
                // This prevents permanent deadlock at the cost of potential positioning issues
                worker_path.waypoints.clear();
                worker_path.current_target = None;
                *worker_state = WorkerState::Idle;
                assigned_sequence.0 = None;
            }
        }
    }
    
    if displaced_count > 0 {
        println!("Emergency displacement system relocated {} stranded workers", displaced_count);
    }
}

/// Find the nearest cell that is part of the valid network within search radius
fn find_nearest_valid_network_cell(
    stranded_pos: (i32, i32),
    network: &NetworkConnectivity,
    max_search_radius: i32,
) -> Option<(i32, i32)> {
    // Use expanding square search pattern for optimal nearest-neighbor finding
    for radius in 1..=max_search_radius {
        // Search in expanding squares around the stranded position
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                // Only check the perimeter of the current radius to avoid redundant checks
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