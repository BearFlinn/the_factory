use std::collections::{HashSet, VecDeque, HashMap};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position},
    materials::{items::{Inventory, InventoryType, InventoryTypes, ItemName}, request_transfer_all_items, request_transfer_specific_items, ItemTransferRequestEvent, RecipeRegistry},
    structures::{Building, WorkersEnRoute, RecipeCrafter},
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
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    mut workers_en_route: Query<&mut WorkersEnRoute>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
    recipe_registry: Res<RecipeRegistry>,
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
        }
    }
}

// Constants for target acquisition scoring
const CONGESTION_PENALTY_NORMAL: i32 = 2;
const CONGESTION_PENALTY_REQUESTER: i32 = 4;
const MAX_ITEM_BONUS: i32 = 20;

fn find_new_target(
    worker_pos: (i32, i32),
    worker_inventory: &Inventory,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    recipe_registry: &RecipeRegistry,
) -> Option<(i32, i32)> {
    if worker_inventory.has_any_item() {
        // Compare distances to both requesters and storage, prefer closer
        let requester_target = find_best_target_for_worker(buildings, workers_en_route, worker_pos, worker_inventory, TargetType::Requester, recipe_registry);
        let storage_target = find_best_target_for_worker(buildings, workers_en_route, worker_pos, worker_inventory, TargetType::Storage, recipe_registry);
        
        match (requester_target, storage_target) {
            (Some(req_pos), Some(stor_pos)) => {
                let req_distance = manhattan_distance_coords(req_pos, worker_pos);
                let stor_distance = manhattan_distance_coords(stor_pos, worker_pos);
                // Prefer requesters if distance is equal (slight bias)
                if req_distance <= stor_distance {
                    Some(req_pos)
                } else {
                    Some(stor_pos)
                }
            }
            (Some(req_pos), None) => Some(req_pos),
            (None, Some(stor_pos)) => Some(stor_pos),
            (None, None) => None,
        }
    } else {
        // Find a sender with items
        find_best_target_for_worker(buildings, workers_en_route, worker_pos, worker_inventory, TargetType::Sender, recipe_registry)
    }
}

#[derive(Clone, Copy)]
enum TargetType {
    Storage,
    Sender,
    Requester,
}

fn find_best_target_for_worker(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    worker_pos: (i32, i32),
    worker_inventory: &Inventory,
    target_type: TargetType,
    recipe_registry: &RecipeRegistry,
) -> Option<(i32, i32)> {
    buildings
        .iter()
        .filter(|(_, _, inventory, inv_type, recipe_crafter)| {
            is_valid_target_for_worker(inventory, inv_type, recipe_crafter, worker_inventory, target_type, recipe_registry)
        })
        .filter_map(|(entity, pos, inventory, _, _)| {
            let score = calculate_target_score(entity, pos, inventory, workers_en_route, worker_pos, target_type);
            score.map(|s| (pos.x, pos.y, s))
        })
        .min_by_key(|(_, _, score)| *score)
        .map(|(x, y, _)| (x, y))
}

fn is_valid_target_for_worker(
    inventory: &Inventory,
    inv_type: &InventoryType,
    recipe_crafter: &Option<&RecipeCrafter>,
    worker_inventory: &Inventory,
    target_type: TargetType,
    recipe_registry: &RecipeRegistry,
) -> bool {
    match target_type {
        TargetType::Storage => matches!(inv_type.0, InventoryTypes::Storage),
        TargetType::Sender => {
            match inv_type.0 {
                InventoryTypes::Sender => inventory.has_any_item(),
                InventoryTypes::Producer => {
                    // Producer acts as sender if it has output items
                    if let Some(crafter) = recipe_crafter {
                        if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                            // Check if we have any output items
                            recipe_def.outputs.keys().any(|output_item| {
                                inventory.get_item_quantity(output_item) > 0
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        TargetType::Requester => {
            match inv_type.0 {
                InventoryTypes::Requester => {
                    if let Some(crafter) = recipe_crafter {
                        if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                            // Check if the worker has any items that this requester needs
                            for (item_name, _) in &recipe_def.inputs {
                                let current_amount = inventory.get_item_quantity(item_name);
                                let worker_has = worker_inventory.get_item_quantity(item_name);
                                
                                // Worker can help if: requester needs items AND worker has them
                                if current_amount < 10 && worker_has > 0 {
                                    return true;
                                }
                            }
                        }
                    }
                    false
                }
                InventoryTypes::Producer => {
                    // Producer acts as requester if it needs input items
                    if let Some(crafter) = recipe_crafter {
                        if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                            // Check if worker has items we need and we have space
                            for (item_name, needed_amount) in &recipe_def.inputs {
                                let current_amount = inventory.get_item_quantity(item_name);
                                let worker_has = worker_inventory.get_item_quantity(item_name);
                                
                                // Need more items and worker has them
                                if current_amount < needed_amount * 10 && worker_has > 0 {
                                    return true;
                                }
                            }
                        }
                    }
                    false
                }
                _ => false,
            }
        }
    }
}

fn calculate_target_score(
    entity: Entity,
    pos: &Position,
    inventory: &Inventory,
    workers_en_route: &Query<&mut WorkersEnRoute>,
    worker_pos: (i32, i32),
    target_type: TargetType,
) -> Option<i32> {
    let congestion = workers_en_route.get(entity).map(|w| w.count).unwrap_or(0);
    let distance = manhattan_distance(pos, worker_pos);
    
    let congestion_penalty = match target_type {
        TargetType::Requester => congestion as i32 * CONGESTION_PENALTY_REQUESTER,
        _ => congestion as i32 * CONGESTION_PENALTY_NORMAL,
    };
    
    let item_bonus = match target_type {
        TargetType::Sender => {
            let item_count = inventory.get_total_quantity() as i32;
            item_count.min(MAX_ITEM_BONUS)
        }
        _ => 0,
    };
    
    Some(distance + congestion_penalty - item_bonus)
}

fn manhattan_distance(pos: &Position, worker_pos: (i32, i32)) -> i32 {
    (pos.x - worker_pos.0).abs() + (pos.y - worker_pos.1).abs()
}

fn manhattan_distance_coords(pos1: (i32, i32), pos2: (i32, i32)) -> i32 {
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

pub fn handle_worker_arrivals(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    inventories: Query<&Inventory>,
    buildings: Query<(Entity, &Position, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    mut workers_en_route: Query<&mut WorkersEnRoute>,
    mut transfer_requests: EventWriter<ItemTransferRequestEvent>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for event in arrival_events.read() {
        if let Some((building_entity, building_inv_type, recipe_crafter)) = buildings.iter()
            .find(|(_, pos, _, _)| pos.x == event.position.0 && pos.y == event.position.1)
            .map(|(entity, _, inv_type, recipe_crafter)| (entity, inv_type, recipe_crafter))
        {
            // Decrement the workers en route counter
            if let Ok(mut en_route) = workers_en_route.get_mut(building_entity) {
                en_route.count = en_route.count.saturating_sub(1);
            }
           
            match building_inv_type.0 {
                InventoryTypes::Storage => {
                    // Worker delivers items to storage building
                    request_transfer_all_items(
                        event.worker,
                        building_entity,
                        &mut transfer_requests,
                        &inventories,
                    );
                }
                InventoryTypes::Sender => {
                    // Building gives items to worker
                    request_transfer_all_items(
                        building_entity,
                        event.worker,
                        &mut transfer_requests,
                        &inventories,
                    );
                }
                InventoryTypes::Requester => {
                    // Worker delivers specific items needed by the requester's recipe
                    if let Some(crafter) = recipe_crafter {
                        if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                            if let Ok(worker_inventory) = inventories.get(event.worker) {
                                let mut needed_items = HashMap::new();
                                
                                // Determine what items are needed based on recipe inputs
                                for (item_name, _) in &recipe_def.inputs {
                                    let worker_has = worker_inventory.get_item_quantity(item_name);
                                    if worker_has > 0 {
                                        needed_items.insert(item_name.clone(), worker_has);
                                    }
                                }
                                
                                if !needed_items.is_empty() {
                                    request_transfer_specific_items(
                                        event.worker,
                                        building_entity,
                                        needed_items,
                                        &mut transfer_requests,
                                    );
                                }
                            }
                        }
                    } else {
                        // Fallback to transferring all items if no recipe
                        request_transfer_all_items(
                            event.worker,
                            building_entity,
                            &mut transfer_requests,
                            &inventories,
                        );
                    }
                }
                InventoryTypes::Producer => {
                    // Producer receives needed inputs and gives away outputs
                    if let Some(crafter) = recipe_crafter {
                        if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                            if let Ok(worker_inventory) = inventories.get(event.worker) {
                                if let Ok(building_inventory) = inventories.get(building_entity) {
                                    let mut items_to_give = HashMap::new();
                                    let mut items_to_take = HashMap::new();
                                    
                                    // Check what inputs the worker can provide
                                    for (item_name, _) in &recipe_def.inputs {
                                        let worker_has = worker_inventory.get_item_quantity(item_name);
                                        if worker_has > 0 {
                                            items_to_give.insert(item_name.clone(), worker_has);
                                        }
                                    }
                                    
                                    // Check what outputs the building can provide
                                    for (item_name, _) in &recipe_def.outputs {
                                        let building_has = building_inventory.get_item_quantity(item_name);
                                        if building_has > 0 {
                                            items_to_take.insert(item_name.clone(), building_has);
                                        }
                                    }
                                    
                                    // Prioritize giving inputs if worker has them
                                    if !items_to_give.is_empty() {
                                        request_transfer_specific_items(
                                            event.worker,
                                            building_entity,
                                            items_to_give,
                                            &mut transfer_requests,
                                        );
                                    } else if !items_to_take.is_empty() {
                                        // Otherwise take outputs
                                        request_transfer_specific_items(
                                            building_entity,
                                            event.worker,
                                            items_to_take,
                                            &mut transfer_requests,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}