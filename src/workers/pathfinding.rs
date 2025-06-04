use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    items::{transfer_items, Inventory},
    structures::{Building, Hub, MultiCellBuilding},
    systems::NetworkConnectivity, workers::{Harvester, Speed, Worker}
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
    mut workers: Query<(Entity, &mut Transform, &mut WorkerPath, &Speed, &Harvester), With<Worker>>,
    harvesters: Query<&Position, With<Building>>,
    hub_query: Query<&MultiCellBuilding, With<Hub>>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    time: Res<Time>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    let hub = hub_query.single();
    
    for (worker_entity, mut transform, mut path, speed, harvester_component) in workers.iter_mut() {
        let Ok(harvester_pos) = harvesters.get(harvester_component.entity) else {
            continue;
        };
        
        // Move toward current target
        if let Some(target) = path.current_target {
            let direction = (target - transform.translation.truncate()).normalize_or_zero();
            let movement = direction * speed.value * time.delta_secs();
            transform.translation += movement.extend(0.0);
            
            // Check if we've arrived (within threshold distance of target center)
            let distance_to_target = (target - transform.translation.truncate()).length();
            
            if distance_to_target <= 1.0 {
                // Fire arrival event
                if let Some(target_coords) = grid.world_to_grid_coordinates(target) {
                    arrival_events.send(WorkerArrivedEvent {
                        worker: worker_entity,
                        position: (target_coords.grid_x, target_coords.grid_y),
                    });
                }
                
                // Arrived at current target, get next waypoint
                path.current_target = path.waypoints.pop_front();
                    
                    // If no more waypoints, calculate new path to opposite destination
                    if path.current_target.is_none() {
                        if let Some (worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                            let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
                            let hub_pos = (hub.center_x, hub.center_y);
                            let harvester_grid_pos = (harvester_pos.x, harvester_pos.y);
                            
                            // Determine destination: if closer to hub, go to harvester; otherwise go to hub
                            let distance_to_hub = ((worker_pos.0 - hub_pos.0).abs() + (worker_pos.1 - hub_pos.1).abs()) as f32;
                            let distance_to_harvester = ((worker_pos.0 - harvester_grid_pos.0).abs() + (worker_pos.1 - harvester_grid_pos.1).abs()) as f32;
                            
                            let destination = if distance_to_hub <= distance_to_harvester {
                                harvester_grid_pos
                            } else {
                                hub_pos
                            };
                            
                            if let Some(new_path) = calculate_path(worker_pos, destination, &network, &grid) {
                                path.waypoints = new_path;
                                path.current_target = path.waypoints.pop_front();
                            }
                        }
                    }
                }
        } else {
            // No current target, calculate initial path to harvester
            if let Some(worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
                let destination = (harvester_pos.x, harvester_pos.y);
                
                if let Some(new_path) = calculate_path(worker_pos, destination, &network, &grid) {
                    path.waypoints = new_path;
                    path.current_target = path.waypoints.pop_front();
                }
            }
        }
    }
}

fn calculate_path(
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

pub fn handle_worker_arrivals(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    mut inventories: Query<&mut Inventory>,
    buildings: Query<(Entity, &Position), With<Building>>,
    hub_query: Query<Entity, With<Hub>>,
) {
    for event in arrival_events.read() {
        // Find building at the arrival position
        let building_at_position = buildings.iter()
            .find(|(_, pos)| pos.x == event.position.0 && pos.y == event.position.1)
            .map(|(entity, _)| entity);
        
        if let Some(building_entity) = building_at_position {
            // Check if this is the hub
            if hub_query.contains(building_entity) {
                // Transfer from worker to hub
                transfer_items(event.worker, building_entity, &mut inventories);
            } else {
                // Transfer from building to worker
                transfer_items(building_entity, event.worker, &mut inventories);
            }
        }
    }
}