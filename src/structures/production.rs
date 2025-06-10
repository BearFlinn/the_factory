use std::collections::HashMap;

use bevy::prelude::*;
use crate::{
    grid::Position, 
    materials::{items::Inventory, InventoryType, InventoryTypes, ItemName, RecipeRegistry}, 
    structures::RecipeCrafter, 
    systems::Operational, 
    workers::tasks::{Priority, Task, TaskTarget}
};

pub fn update_recipe_crafters(
    mut query: Query<(&mut RecipeCrafter, &Operational, &mut Inventory)>,
    recipe_registry: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut crafter, operational, mut inventory) in query.iter_mut() {
        if !operational.0 {
            continue;
        }
        
        if crafter.timer.tick(time.delta()).just_finished() {
            if let Some(recipe) = recipe_registry.get_definition(&crafter.recipe) {
                // Check if we have all required inputs
                let can_craft = !inventory.is_full() || recipe.inputs.iter().all(|(item_name, quantity)| {
                    inventory.has_at_least(item_name, *quantity)
                });
                
                if can_craft {
                    // Consume inputs
                    for (item_name, quantity) in &recipe.inputs {
                        inventory.remove_item(item_name, *quantity);
                    }
                    
                    // Produce outputs
                    for (item_name, quantity) in &recipe.outputs {
                        inventory.add_item(item_name, *quantity);
                    }
                }
            }
            
            crafter.timer.reset();
        }
    }
}

#[derive(Event)]
pub struct CrafterLogisticsRequest {
    pub crafter: Entity,
    pub position: Position,
    pub needs: Option<HashMap<ItemName, u32>>,
    pub has: Option<HashMap<ItemName, u32>>,
}

pub fn crafter_logistics_requests(
    mut crafters: Query<(Entity, &mut RecipeCrafter, &Inventory, &InventoryType, &Position), Changed<Inventory>>,
    tasks: Query<(Entity, &TaskTarget, &mut Priority), With<Task>>,
    mut events: EventWriter<CrafterLogisticsRequest>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (crafter_entity, crafter, inventory, inv_type, position) in crafters.iter_mut() {
        if !tasks.iter().any(|(_, target_entity, _)| target_entity.0 == crafter_entity) {
            match inv_type.0 {
                InventoryTypes::Sender => {
                    if inventory.get_total_quantity() > 10 {
                        events.send(CrafterLogisticsRequest {
                            crafter: crafter_entity,
                            position: position.clone(),
                            needs: None,
                            has: Some(inventory.get_all_items()),
                        });
                    }

                }
                InventoryTypes::Requester => {
                    if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                        let required_items: HashMap<_, _> = recipe_def.inputs.iter().map(|(item, quantity)| (item.clone(), quantity * 3)).collect();
                        if !inventory.has_items_for_recipe(&required_items) {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: position.clone(),
                                needs: Some(required_items.clone()),
                                has: None,
                            });
                        }
                    }
                }
                InventoryTypes::Producer => {
                    if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                        let required_items: HashMap<_, _> = recipe_def.inputs.iter().map(|(item, quantity)| (item.clone(), quantity * 3)).collect();
                        if !inventory.has_items_for_recipe(&required_items) {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: position.clone(),
                                needs: Some(required_items.clone()),
                                has: None,
                            });
                        }
                        let produced_items: HashMap<_, _> = recipe_def.outputs.iter().map(|(item, quantity)| (item.clone(), quantity * 20)).collect();
                        if !inventory.has_items_for_recipe(&produced_items) {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: position.clone(),
                                needs: None,
                                has: Some(produced_items),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}