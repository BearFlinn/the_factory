use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    items::{transfer_items, Inventory, InventoryType, InventoryTypes},
    structures::Building,
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
    mut workers: Query<(Entity, &mut Transform, &mut WorkerPath, &Speed, &Inventory), With<Worker>>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    time: Res<Time>,
    mut arrival_events: EventWriter<WorkerArrivedEvent>,
) {
    for (worker_entity, mut transform, mut path, speed, worker_inventory) in workers.iter_mut() {
        // Move toward current target
        if let Some(target) = path.current_target {
            let direction = (target - transform.translation.truncate()).normalize_or_zero();
            let movement = direction * speed.value * time.delta_secs();
            transform.translation += movement.extend(0.0);
            
            // Check if we've arrived (within threshold distance of target center)
            let distance_to_target = (target - transform.translation.truncate()).length();
            
            if distance_to_target <= 1.0 {

                // Arrived at current target, get next waypoint
                path.current_target = path.waypoints.pop_front();
                    

                if path.current_target.is_none() {
                    if let Some(target_coords) = grid.world_to_grid_coordinates(target) {
                        arrival_events.send(WorkerArrivedEvent {
                            worker: worker_entity,
                            position: (target_coords.grid_x, target_coords.grid_y),
                        });
                    }
                    continue;
                }
            }
        } else {
            // No current target, calculate new path
            if let Some(worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
                
                
                let destination = find_new_target(worker_pos, worker_inventory, &buildings);
                
                if let Some(dest_pos) = destination {
                    if let Some(new_path) = calculate_path(worker_pos, dest_pos, &network, &grid) {
                        path.waypoints = new_path;
                        path.current_target = path.waypoints.pop_front();
                    }
                }
            }
        }
    }
}

fn find_new_target(
    worker_pos: (i32, i32),
    worker_inventory: &Inventory,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
) -> Option<(i32, i32)> {
    // Determine destination based on worker inventory state
    let destination = if worker_inventory.has_item(0, 1) {
        if let Some((x, y)) = find_nearest_empty_requester(buildings, worker_pos) {
            return Some((x, y));
        } else {
            find_nearest_building_by_type(buildings, worker_pos, InventoryTypes::Storage)
        }

    } else {
        // Worker is empty - find nearest sender with items
        find_nearest_sender_with_items(buildings, worker_pos)
    };
    
    destination
}

fn find_nearest_building_by_type(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    worker_pos: (i32, i32),
    target_type: InventoryTypes,
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, _, inv_type)| inv_type.0 == target_type)
        .map(|(_, pos, _, _)| (pos.x, pos.y))
        .min_by_key(|(x, y)| {
            let dx = (*x - worker_pos.0).abs();
            let dy = (*y - worker_pos.1).abs();
            dx + dy // Manhattan distance
        })
}

// Helper function to find nearest sender with available items
fn find_nearest_sender_with_items(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    worker_pos: (i32, i32),
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, inventory, inv_type)| {
            matches!(inv_type.0, InventoryTypes::Sender) && inventory.has_item(0, 1)
        })
        .map(|(_, pos, _, _)| (pos.x, pos.y))
        .min_by_key(|(x, y)| {
            let dx = (*x - worker_pos.0).abs();
            let dy = (*y - worker_pos.1).abs();
            dx + dy // Manhattan distance
        })
}

fn find_nearest_empty_requester(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    worker_pos: (i32, i32),
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, inventory, inv_type)| {
            matches!(inv_type.0, InventoryTypes::Requester) && !inventory.has_item(0, 5)
        })
        .map(|(_, pos, _, _)| (pos.x, pos.y))
        .min_by_key(|(x, y)| {
            let dx = (*x - worker_pos.0).abs();
            let dy = (*y - worker_pos.1).abs();
            dx + dy // Manhattan distance
        })
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
    buildings: Query<(Entity, &Position, &InventoryType), With<Building>>,
) {
    for event in arrival_events.read() {
        // Find building at the arrival position
        let building_at_position = buildings.iter()
            .find(|(_, pos, _)| pos.x == event.position.0 && pos.y == event.position.1)
            .map(|(entity, _, inv_type)| (entity, inv_type));
        
        if let Some((building_entity, building_inv_type)) = building_at_position {
            match building_inv_type.0 {
                InventoryTypes::Storage => {
                    // Transfer from worker to storage
                    transfer_items(event.worker, building_entity, &mut inventories);
                }
                InventoryTypes::Sender => {
                    // Transfer from sender to worker
                    transfer_items(building_entity, event.worker, &mut inventories);
                }
                InventoryTypes::Requester => {
                    // Transfer from worker to requester
                    transfer_items(event.worker, building_entity, &mut inventories);
                }
                _ => {} // No transfer for other types yet
            }
        }
    }
}
