use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    materials::items::{transfer_items, Inventory, InventoryType, InventoryTypes},
    structures::{Building, WorkersEnRoute},
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
    mut workers_en_route: Query<&mut WorkersEnRoute>,
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
            
            let distance_to_target = (target - transform.translation.truncate()).length();
            
            if distance_to_target <= 1.0 {
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
                
                let destination = find_new_target(worker_pos, worker_inventory, &buildings, &workers_en_route);
                
                if let Some(dest_pos) = destination {
                    if let Some(new_path) = calculate_path(worker_pos, dest_pos, &network, &grid) {
                        path.waypoints = new_path;
                        path.current_target = path.waypoints.pop_front();
                        
                        // Reserve the target building
                        if let Some(building_entity) = buildings.iter()
                            .find(|(_, pos, _, _)| pos.x == dest_pos.0 && pos.y == dest_pos.1)
                            .map(|(entity, _, _, _)| entity) 
                        {
                            if let Ok(mut en_route) = workers_en_route.get_mut(building_entity) {
                                en_route.count += 1;
                            }
                        }
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
    workers_en_route: &Query<&mut WorkersEnRoute>,
) -> Option<(i32, i32)> {
    let destination = if worker_inventory.has_item(0, 1) {
        if let Some((x, y)) = find_nearest_empty_requester_with_congestion(buildings, workers_en_route, worker_pos) {
            return Some((x, y));
        } else {
            find_nearest_building_by_type_with_congestion(buildings, workers_en_route, worker_pos, InventoryTypes::Storage)
        }
    } else {
        find_nearest_sender_with_items_and_congestion(buildings, workers_en_route, worker_pos)
    };
    
    destination
}

// Updated helper functions that factor in congestion
fn find_nearest_building_by_type_with_congestion(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    worker_pos: (i32, i32),
    target_type: InventoryTypes,
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, _, inv_type)| inv_type.0 == target_type)
        .map(|(entity, pos, _, _)| {
            let congestion = workers_en_route.get(entity).map(|w| w.count).unwrap_or(0);
            let distance = (pos.x - worker_pos.0).abs() + (pos.y - worker_pos.1).abs();
            let score = distance + (congestion as i32 * 2); // Congestion penalty
            (pos.x, pos.y, score)
        })
        .min_by_key(|(_, _, score)| *score)
        .map(|(x, y, _)| (x, y))
}

fn find_nearest_sender_with_items_and_congestion(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    worker_pos: (i32, i32),
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, inventory, inv_type)| {
            matches!(inv_type.0, InventoryTypes::Sender) && inventory.has_item(0, 1)
        })
        .map(|(entity, pos, inventory, _)| {
            let congestion = workers_en_route.get(entity).map(|w| w.count).unwrap_or(0);
            let distance = (pos.x - worker_pos.0).abs() + (pos.y - worker_pos.1).abs();
            let item_count = inventory.get_item_quantity(0); // Get actual ore count
            let item_bonus = (item_count as i32).min(20); // Cap bonus at 20 to prevent overflow
            let score = distance + (congestion as i32 * 2) - item_bonus; // Subtract item count for priority
            (pos.x, pos.y, score)
        })
        .min_by_key(|(_, _, score)| *score)
        .map(|(x, y, _)| (x, y))
}

fn find_nearest_empty_requester_with_congestion(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    worker_pos: (i32, i32),
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, inventory, inv_type)| {
            matches!(inv_type.0, InventoryTypes::Requester) && !inventory.has_item(0, 5)
        })
        .map(|(entity, pos, _, _)| {
            let congestion = workers_en_route.get(entity).map(|w| w.count).unwrap_or(0);
            let distance = (pos.x - worker_pos.0).abs() + (pos.y - worker_pos.1).abs();
            let score = distance + (congestion as i32 * 4); // Higher penalty for requesters
            (pos.x, pos.y, score)
        })
        .min_by_key(|(_, _, score)| *score)
        .map(|(x, y, _)| (x, y))
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
    mut workers_en_route: Query<&mut WorkersEnRoute>,
) {
    for event in arrival_events.read() {
        if let Some((building_entity, building_inv_type)) = buildings.iter()
            .find(|(_, pos, _)| pos.x == event.position.0 && pos.y == event.position.1)
            .map(|(entity, _, inv_type)| (entity, inv_type)) 
        {
            // Decrement the workers en route counter
            if let Ok(mut en_route) = workers_en_route.get_mut(building_entity) {
                en_route.count = en_route.count.saturating_sub(1);
            }
            
            match building_inv_type.0 {
                InventoryTypes::Storage => {
                    transfer_items(event.worker, building_entity, &mut inventories);
                }
                InventoryTypes::Sender => {
                    transfer_items(building_entity, event.worker, &mut inventories);
                }
                InventoryTypes::Requester => {
                    transfer_items(event.worker, building_entity, &mut inventories);
                }
                _ => {}
            }
        }
    }
}
