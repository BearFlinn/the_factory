use bevy::prelude::*;
use crate::{
    materials::{items::Inventory, RecipeRegistry}, structures::{ RecipeCrafter}, systems::Operational
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
                let can_craft = inventory.is_full() || recipe.inputs.iter().all(|(item_name, quantity)| {
                    inventory.has_item(item_name, *quantity)
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