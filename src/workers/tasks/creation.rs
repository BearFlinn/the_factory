use std::collections::HashMap;
use bevy::prelude::*;
use crate::{
    grid::Position,
    materials::{Inventory, InventoryType, InventoryTypes, ItemName, RecipeRegistry},
    structures::{Building, ConstructionMaterialRequest, CrafterLogisticsRequest, RecipeCrafter},
    workers::manhattan_distance_coords,
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
            let mut all_tasks = Vec::new();
            
            // Create pickup/dropoff task pairs for each supply source
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
                
                all_tasks.push(pickup_task);
                all_tasks.push(dropoff_task);
            }
            
            // Create task sequence for all construction deliveries
            if !all_tasks.is_empty() {
                let sequence_entity = commands.spawn(
                    TaskSequenceBundle::new(all_tasks.clone(), request.priority.clone())
                ).id();
                
                // Link all tasks to the sequence
                for task_id in all_tasks {
                    commands.entity(task_id).insert(SequenceMember(sequence_entity));
                }
            }
        }
    }
}

fn calculate_supply_plan(
    requester_pos: (i32, i32),
    needed_items: &HashMap<ItemName, u32>,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    const WORKER_CAPACITY: u32 = 20; // Match worker inventory capacity
    
    let mut remaining_needs = needed_items.clone();
    let mut supply_plan = Vec::new();
    
    while !remaining_needs.is_empty() {
        let mut best_contribution: Option<(Entity, Position, HashMap<ItemName, u32>)> = None;
        let mut best_distance = i32::MAX;
        
        // Find the closest building that can contribute something we still need
        for (entity, pos, inventory, inv_type, _) in buildings.iter() {
            if inv_type.0 != InventoryTypes::Storage && inv_type.0 != InventoryTypes::Sender {
                continue;
            }
            
            let mut contribution = HashMap::new();
            
            // Calculate what this building can actually contribute
            for (item_name, &still_needed) in remaining_needs.iter() {
                let available = inventory.get_item_quantity(item_name);
                if available > 0 {
                    let can_contribute = available.min(still_needed);
                    contribution.insert(item_name.clone(), can_contribute);
                }
            }
            
            if contribution.is_empty() {
                continue;
            }
            
            let distance = manhattan_distance_coords(requester_pos, (pos.x, pos.y));
            if distance < best_distance {
                best_distance = distance;
                best_contribution = Some((entity, *pos, contribution));
            }
        }
        
        // Process the best contribution with capacity chunking
        if let Some((entity, pos, contribution)) = best_contribution {
            // Split contribution into worker-capacity-sized chunks
            let chunked_contributions = chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);
            
            // Add each chunk as a separate trip to the supply plan
            for chunk in chunked_contributions {
                supply_plan.push((entity, pos, chunk.clone()));
                
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
            // No building can contribute anything we need
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