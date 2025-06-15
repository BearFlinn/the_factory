use std::collections::HashMap;
use bevy::prelude::*;
use crate::{
    grid::Position,
    materials::{Inventory, InventoryType, InventoryTypes, ItemName, RecipeRegistry},
    structures::{Building, ConstructionMaterialRequest, CrafterLogisticsRequest, RecipeCrafter},
    workers::{manhattan_distance_coords, Worker, WorkerState},
};
use super::components::*;

pub fn create_logistics_tasks(
    mut commands: Commands,
    mut events: EventReader<CrafterLogisticsRequest>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    recipes: Res<RecipeRegistry>,
) {
    for event in events.read() {
        match (&event.needs, &event.has) {
            (Some(needed_items), None) => {
                
                let supply_plan = calculate_supply_plan(
                    (event.position.x, event.position.y),
                    needed_items,
                    &buildings,
                );
                
                if !supply_plan.is_empty() {
                    let mut all_tasks = Vec::new();
                    
                    for (building_entity, building_pos, items_to_pickup) in supply_plan {
                        let pickup_task = commands.spawn((
                            TaskBundle::new(
                                building_entity,
                                building_pos,
                                TaskAction::Pickup(Some(items_to_pickup.clone())),
                                event.priority.clone()
                            ),
                        )).id();
                        
                        let dropoff_task = commands.spawn((
                            TaskBundle::new(
                                event.crafter,
                                event.position,
                                TaskAction::Dropoff(Some(items_to_pickup)),
                                event.priority.clone()
                            ),
                        )).id();
                        
                        all_tasks.push(pickup_task);
                        all_tasks.push(dropoff_task);
                    }
                    
                    if !all_tasks.is_empty() {
                        let sequence_entity = commands.spawn(
                            TaskSequenceBundle::new(all_tasks.clone(), Priority::Medium)
                        ).id();
                        
                        for task_id in all_tasks {
                            commands.entity(task_id).insert(SequenceMember(sequence_entity));
                        }
                    }
                }
            }
            (None, Some(excess_items)) => {
                let pickup_task = commands.spawn((
                    TaskBundle::new(
                        event.crafter,
                        event.position,
                        TaskAction::Pickup(Some(excess_items.clone())),
                        event.priority.clone(),
                    ),
                )).id();
                
                if let Some((receiver_entity, receiver_pos)) = find_closest_storage_receiver(
                    (event.position.x, event.position.y),
                    excess_items,
                    &buildings,
                    &recipes
                ) {
                    let dropoff_task = commands.spawn((
                        TaskBundle::new(
                            receiver_entity,
                            receiver_pos,
                            TaskAction::Dropoff(None),
                            event.priority.clone()
                        ),
                    )).id();
                    
                    let sequence_entity = commands.spawn(
                        TaskSequenceBundle::new(
                            vec![pickup_task, dropoff_task],
                            event.priority.clone()
                        )
                    ).id();
                    
                    commands.entity(pickup_task).insert(SequenceMember(sequence_entity));
                    commands.entity(dropoff_task).insert(SequenceMember(sequence_entity));
                } else {
                    commands.entity(pickup_task).despawn();
                }
            }
            _ => {}
        }
    }
}

pub fn create_construction_logistics_tasks(
    mut commands: Commands,
    mut construction_requests: EventReader<ConstructionMaterialRequest>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
) {
    for request in construction_requests.read() {
        // Calculate supply plan for construction materials
        let supply_plan = calculate_supply_plan(
            (request.position.x, request.position.y),
            &request.needed_materials,
            &buildings,
        );
        
        if !supply_plan.is_empty() {
            // Create separate task sequences for each supplier to enable parallel work
            for (supplier_entity, supplier_pos, items_to_pickup) in supply_plan {
                let pickup_task = commands.spawn((
                    TaskBundle::new(
                        supplier_entity,
                        supplier_pos,
                        TaskAction::Pickup(Some(items_to_pickup.clone())),
                        request.priority.clone()
                    ),
                )).id();
                
                let dropoff_task = commands.spawn((
                    TaskBundle::new(
                        request.site,
                        request.position,
                        TaskAction::Dropoff(Some(items_to_pickup)),
                        request.priority.clone()
                    ),
                )).id();
                
                // Create individual sequence for each supplier (enables parallel work)
                let sequence_entity = commands.spawn(
                    TaskSequenceBundle::new(
                        vec![pickup_task, dropoff_task], 
                        request.priority.clone()
                    )
                ).id();
                
                // Link tasks to their sequence
                commands.entity(pickup_task).insert(SequenceMember(sequence_entity));
                commands.entity(dropoff_task).insert(SequenceMember(sequence_entity));
            }
        }
    }
}

fn calculate_supply_plan(
    requester_pos: (i32, i32),
    needed_items: &HashMap<ItemName, u32>,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    const WORKER_CAPACITY: u32 = 20;
    
    let mut remaining_needs = needed_items.clone();
    let mut supply_plan = Vec::new();
    // Track reserved items per supplier to prevent over-allocation
    let mut reserved_items: HashMap<Entity, HashMap<ItemName, u32>> = HashMap::new();
    
    while !remaining_needs.is_empty() {
        let mut best_supplier: Option<(Entity, Position, HashMap<ItemName, u32>, f32)> = None;
        
        // Evaluate all potential suppliers and pick the best one
        for (entity, pos, inventory, inv_type, _) in buildings.iter() {
            if inv_type.0 != InventoryTypes::Storage && inv_type.0 != InventoryTypes::Sender {
                continue;
            }
            
            let mut contribution = HashMap::new();
            let mut total_contribution_value = 0u32;
            
            // Get already reserved amounts for this supplier
            let reserved_for_entity = reserved_items.get(&entity).cloned().unwrap_or_default();
            
            // Calculate what this building can actually contribute (accounting for reservations)
            for (item_name, &still_needed) in remaining_needs.iter() {
                let total_available = inventory.get_item_quantity(item_name);
                let already_reserved = reserved_for_entity.get(item_name).copied().unwrap_or(0);
                let available = total_available.saturating_sub(already_reserved);
                
                if available > 0 {
                    let can_contribute = available.min(still_needed);
                    contribution.insert(item_name.clone(), can_contribute);
                    total_contribution_value += can_contribute;
                }
            }
            
            if contribution.is_empty() {
                continue;
            }
            
            // Score supplier based on contribution value vs distance
            let distance = manhattan_distance_coords(requester_pos, (pos.x, pos.y));
            let efficiency_score = total_contribution_value as f32 / (distance as f32 + 1.0);
            
            // Prefer suppliers that can provide substantial amounts
            let substantial_bonus = if total_contribution_value >= WORKER_CAPACITY { 2.0 } else { 1.0 };
            let final_score = efficiency_score * substantial_bonus;
            
            if best_supplier.as_ref().map_or(true, |(_, _, _, score)| final_score > *score) {
                best_supplier = Some((entity, *pos, contribution, final_score));
            }
        }
        
        // Process the best supplier with capacity chunking
        if let Some((entity, pos, contribution, _)) = best_supplier {
            let chunked_contributions = chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);
            
            for chunk in chunked_contributions {
                supply_plan.push((entity, pos, chunk.clone()));
                
                // Reserve items from this supplier
                let reserved_for_entity = reserved_items.entry(entity).or_insert_with(HashMap::new);
                for (item_name, contributed_amount) in &chunk {
                    *reserved_for_entity.entry(item_name.clone()).or_insert(0) += contributed_amount;
                }
                
                // Subtract this chunk from remaining needs
                for (item_name, contributed_amount) in &chunk {
                    if let Some(still_needed) = remaining_needs.get_mut(item_name) {
                        *still_needed = still_needed.saturating_sub(*contributed_amount);
                        if *still_needed == 0 {
                            remaining_needs.remove(item_name);
                        }
                    }
                }
            }
        } else {
            break;
        }
    }
    
    supply_plan
}

/// Splits a contribution into multiple chunks that fit within worker capacity
fn chunk_contribution_by_capacity(
    contribution: HashMap<ItemName, u32>,
    capacity: u32,
) -> Vec<HashMap<ItemName, u32>> {
    let mut chunks = Vec::new();
    let mut remaining_items = contribution;
    
    while !remaining_items.is_empty() {
        let mut current_chunk = HashMap::new();
        let mut current_chunk_size = 0;
        
        // Fill current chunk up to capacity
        let mut items_to_remove = Vec::new();
        
        for (item_name, quantity) in remaining_items.iter_mut() {
            if current_chunk_size >= capacity {
                break;
            }
            
            let available_space = capacity - current_chunk_size;
            let items_to_take = (*quantity).min(available_space);
            
            if items_to_take > 0 {
                current_chunk.insert(item_name.clone(), items_to_take);
                current_chunk_size += items_to_take;
                *quantity -= items_to_take;
                
                if *quantity == 0 {
                    items_to_remove.push(item_name.clone());
                }
            }
        }
        
        // Remove depleted items
        for item_name in items_to_remove {
            remaining_items.remove(&item_name);
        }
        
        // Add completed chunk to results
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        // Safety check to prevent infinite loops
        if current_chunk_size == 0 {
            break;
        }
    }
    
    chunks
}

fn find_closest_storage_receiver(
    position: (i32, i32),
    _items: &HashMap<ItemName, u32>,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    _recipes: &Res<RecipeRegistry>,
) -> Option<(Entity, Position)> {
    let mut closest_building = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, pos, inv, inv_type, _) in buildings.iter() {
        if inv_type.0 == InventoryTypes::Storage {
            let distance = manhattan_distance_coords((pos.x, pos.y), position);
            if distance < closest_distance && !inv.is_full() {
                closest_building = Some((entity, *pos));
                closest_distance = distance;
            }
        }
    }
    
    closest_building
}

#[derive(Resource)]
pub struct ProactiveTaskTimer {
    timer: Timer,
}

impl Default for ProactiveTaskTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        }
    }
}

pub fn create_proactive_tasks(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ProactiveTaskTimer>,
    idle_workers: Query<Entity, (With<Worker>, With<WorkerState>)>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    recipe_crafters: Query<&RecipeCrafter>,
    recipe_registry: Res<RecipeRegistry>,
    existing_tasks: Query<&TaskTarget, With<Task>>,
) {
    // Only run periodically
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }
    
    // Count idle workers to determine how many proactive tasks to create
    let idle_count = idle_workers.iter()
        .filter(|&worker| {
            // Additional check would go here to verify worker is truly idle
            // This is simplified for the example
            true
        })
        .count();
    
    if idle_count == 0 {
        return;
    }
    
    // Get existing task targets to avoid conflicts
    let existing_targets: std::collections::HashSet<Entity> = existing_tasks
        .iter()
        .map(|target| target.0)
        .collect();
    
    let mut proactive_sequences = Vec::new();
    
    // 1. Sender → Storage optimization
    proactive_sequences.extend(identify_sender_to_storage_tasks(
        &buildings, 
        &existing_targets,
        idle_count / 2 // Limit to prevent spam
    ));
    
    // 2. Storage → Requester proactive restocking
    proactive_sequences.extend(identify_storage_to_requester_tasks(
        &buildings,
        &recipe_crafters,
        &recipe_registry,
        &existing_targets,
        idle_count / 2
    ));
    
    // 3. Storage load balancing
    proactive_sequences.extend(identify_storage_balancing_tasks(
        &buildings,
        &existing_targets,
        idle_count / 2
    ));
    
    // Create the task sequences
    for (pickup_building, dropoff_building, items) in proactive_sequences.into_iter().take(idle_count) {
        let pickup_pos = buildings.get(pickup_building).unwrap().1;
        let dropoff_pos = buildings.get(dropoff_building).unwrap().1;
        
        let pickup_task = commands.spawn(TaskBundle::new(
            pickup_building,
            *pickup_pos,
            TaskAction::Pickup(Some(items.clone())),
            Priority::Low, // Key: Low priority for proactive tasks
        )).id();
        
        let dropoff_task = commands.spawn(TaskBundle::new(
            dropoff_building,
            *dropoff_pos,
            TaskAction::Dropoff(Some(items)),
            Priority::Low,
        )).id();
        
        let sequence_entity = commands.spawn(TaskSequenceBundle::new(
            vec![pickup_task, dropoff_task],
            Priority::Low,
        )).id();
        
        commands.entity(pickup_task).insert(SequenceMember(sequence_entity));
        commands.entity(dropoff_task).insert(SequenceMember(sequence_entity));
    }
}

fn identify_sender_to_storage_tasks(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    existing_targets: &std::collections::HashSet<Entity>,
    max_tasks: usize,
) -> Vec<(Entity, Entity, HashMap<ItemName, u32>)> {
    let mut opportunities = Vec::new();
    
    // Find senders with items
    let senders: Vec<_> = buildings.iter()
        .filter(|(entity, _, inventory, inv_type)| {
            inv_type.0 == InventoryTypes::Sender 
                && !inventory.is_empty()
                && !existing_targets.contains(entity)
        })
        .collect();
    
    // Find storage with space
    let storage_buildings: Vec<_> = buildings.iter()
        .filter(|(entity, _, inventory, inv_type)| {
            inv_type.0 == InventoryTypes::Storage 
                && !inventory.is_full()
                && !existing_targets.contains(entity)
        })
        .collect();
    
    for (sender_entity, sender_pos, sender_inv, _) in senders {
        let sender_pos_tuple = (sender_pos.x, sender_pos.y);
        
        // Find closest storage with space
        let closest_storage = storage_buildings.iter()
            .min_by_key(|(_, storage_pos, _, _)| {
                manhattan_distance_coords(sender_pos_tuple, (storage_pos.x, storage_pos.y))
            });
        
        if let Some((storage_entity, _, storage_inv, _)) = closest_storage {
            // Calculate how much we can move (respecting worker capacity)
            let items_to_move = calculate_feasible_transfer(
                sender_inv, 
                storage_inv, 
                20 // Worker capacity
            );
            
            if !items_to_move.is_empty() {
                opportunities.push((sender_entity, *storage_entity, items_to_move));
            }
        }
        
        if opportunities.len() >= max_tasks {
            break;
        }
    }
    
    opportunities
}

fn identify_storage_to_requester_tasks(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    recipe_crafters: &Query<&RecipeCrafter>,
    recipe_registry: &Res<RecipeRegistry>,
    existing_targets: &std::collections::HashSet<Entity>,
    max_tasks: usize,
) -> Vec<(Entity, Entity, HashMap<ItemName, u32>)> {
    let mut opportunities = Vec::new();
    
    // Find requesters that are low on items AND have defined recipes
    let requesters: Vec<_> = buildings.iter()
        .filter(|(entity, _, inventory, inv_type)| {
            inv_type.0 == InventoryTypes::Requester 
                && inventory.get_total_quantity() < 50 // Low threshold
                && !existing_targets.contains(entity)
                && recipe_crafters.get(*entity).is_ok() // Must have a recipe
        })
        .collect();
    
    let storage_buildings: Vec<_> = buildings.iter()
        .filter(|(entity, _, inventory, inv_type)| {
            inv_type.0 == InventoryTypes::Storage 
                && !inventory.is_empty()
                && !existing_targets.contains(entity)
        })
        .collect();
    
    for (requester_entity, requester_pos, requester_inv, _) in requesters {
        let requester_pos_tuple = (requester_pos.x, requester_pos.y);
        
        // Get the recipe for this requester
        let recipe_crafter = recipe_crafters.get(requester_entity).unwrap();
        let Some(recipe_name) = recipe_crafter.get_active_recipe() else {
            continue;
        };
        let recipe_def = match recipe_registry.get_definition(recipe_name) {
            Some(def) => def,
            None => continue, // Skip if recipe not found
        };
        
        // Find closest storage with useful items FOR THIS RECIPE
        for (storage_entity, storage_pos, storage_inv, _) in &storage_buildings {
            let distance = manhattan_distance_coords(
                requester_pos_tuple, 
                (storage_pos.x, storage_pos.y)
            );
            
            // Only consider nearby storage (within 10 tiles)
            if distance <= 10 {
                let items_to_move = calculate_recipe_aware_restock(
                    storage_inv,
                    requester_inv,
                    &recipe_def.inputs, // Only move recipe inputs
                    20 // Worker capacity
                );
                
                if !items_to_move.is_empty() {
                    opportunities.push((
                        *storage_entity, 
                        requester_entity, 
                        items_to_move
                    ));
                    break; // One task per requester
                }
            }
        }
        
        if opportunities.len() >= max_tasks {
            break;
        }
    }
    
    opportunities
}

fn identify_storage_balancing_tasks(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType), With<Building>>,
    existing_targets: &std::collections::HashSet<Entity>,
    max_tasks: usize,
) -> Vec<(Entity, Entity, HashMap<ItemName, u32>)> {
    let mut opportunities = Vec::new();
    
    let storage_buildings: Vec<_> = buildings.iter()
        .filter(|(entity, _, _, inv_type)| {
            inv_type.0 == InventoryTypes::Storage && !existing_targets.contains(entity)
        })
        .collect();
    
    // Find storage pairs where one is much fuller than the other
    for i in 0..storage_buildings.len() {
        for j in (i + 1)..storage_buildings.len() {
            let (entity1, pos1, inv1, _) = &storage_buildings[i];
            let (entity2, pos2, inv2, _) = &storage_buildings[j];
            
            let fullness1 = inv1.get_total_quantity() as f32 / inv1.capacity as f32;
            let fullness2 = inv2.get_total_quantity() as f32 / inv2.capacity as f32;
            
            // If one is >80% full and the other is <50% full
            if (fullness1 > 0.8 && fullness2 < 0.5) || (fullness2 > 0.8 && fullness1 < 0.5) {
                let distance = manhattan_distance_coords(
                    (pos1.x, pos1.y), 
                    (pos2.x, pos2.y)
                );
                
                // Only balance nearby storage (within 15 tiles)
                if distance <= 15 {
                    let (from_entity, to_entity, items) = if fullness1 > fullness2 {
                        let items = calculate_balancing_transfer(inv1, inv2, 20);
                        (*entity1, *entity2, items)
                    } else {
                        let items = calculate_balancing_transfer(inv2, inv1, 20);
                        (*entity2, *entity1, items)
                    };
                    
                    if !items.is_empty() {
                        opportunities.push((from_entity, to_entity, items));
                    }
                }
            }
        }
        
        if opportunities.len() >= max_tasks {
            break;
        }
    }
    
    opportunities
}

// Helper functions for transfer calculations

fn calculate_feasible_transfer(
    sender_inv: &Inventory,
    receiver_inv: &Inventory,
    worker_capacity: u32,
) -> HashMap<ItemName, u32> {
    let mut transfer = HashMap::new();
    let mut total_transfer = 0;
    
    let receiver_space = receiver_inv.capacity - receiver_inv.get_total_quantity();
    let max_transfer = worker_capacity.min(receiver_space);
    
    for (item_name, &quantity) in &sender_inv.items {
        if total_transfer >= max_transfer {
            break;
        }
        
        let transfer_amount = quantity.min(max_transfer - total_transfer);
        if transfer_amount > 0 {
            transfer.insert(item_name.clone(), transfer_amount);
            total_transfer += transfer_amount;
        }
    }
    
    transfer
}

fn calculate_recipe_aware_restock(
    storage_inv: &Inventory,
    requester_inv: &Inventory,
    recipe_inputs: &HashMap<ItemName, u32>,
    worker_capacity: u32,
) -> HashMap<ItemName, u32> {
    let mut transfer = HashMap::new();
    let mut total_transfer = 0;
    
    let requester_space = requester_inv.capacity - requester_inv.get_total_quantity();
    let max_transfer = worker_capacity.min(requester_space);
    
    // Only consider items that are actually recipe inputs
    for (recipe_item, _recipe_quantity) in recipe_inputs {
        if total_transfer >= max_transfer {
            break;
        }
        
        let storage_quantity = storage_inv.get_item_quantity(recipe_item);
        let requester_quantity = requester_inv.get_item_quantity(recipe_item);
        
        // Only transfer if:
        // 1. Storage has the item
        // 2. Storage has significantly more than requester
        // 3. Requester could benefit from more of this recipe input
        if storage_quantity > 0 && storage_quantity > requester_quantity + 5 {
            let transfer_amount = (storage_quantity / 3) // Conservative transfer
                .min(max_transfer - total_transfer)
                .min(storage_quantity)
                .max(1); // Minimum 1 if conditions are met
            
            if transfer_amount > 0 {
                transfer.insert(recipe_item.clone(), transfer_amount);
                total_transfer += transfer_amount;
            }
        }
    }
    
    transfer
}

fn calculate_balancing_transfer(
    full_inv: &Inventory,
    empty_inv: &Inventory,
    worker_capacity: u32,
) -> HashMap<ItemName, u32> {
    let mut transfer = HashMap::new();
    let mut total_transfer = 0;
    
    let empty_space = empty_inv.capacity - empty_inv.get_total_quantity();
    let max_transfer = worker_capacity.min(empty_space);
    
    for (item_name, &quantity) in &full_inv.items {
        if total_transfer >= max_transfer {
            break;
        }
        
        // Transfer up to half of the quantity
        let transfer_amount = (quantity / 2)
            .min(max_transfer - total_transfer);
        
        if transfer_amount > 0 {
            transfer.insert(item_name.clone(), transfer_amount);
            total_transfer += transfer_amount;
        }
    }
    
    transfer
}