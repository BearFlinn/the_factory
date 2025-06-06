use bevy::prelude::*;
use crate::{
    grid::Position, materials::{Inventory, RecipeRegistry}, structures::{Building, PowerConsumer, PowerGenerator, RecipeCrafter}, systems::{NetworkConnectivity, PowerGrid}
};

#[derive(Component)]
pub struct Operational(pub bool);

pub fn update_crafter_operational_status(
    mut query: Query<(&RecipeCrafter, &mut Operational, &Inventory)>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (crafter, mut operational, inventory) in query.iter_mut() {
        if let Some(recipe) = recipe_registry.get_definition(crafter.recipe) {
            let has_inputs = recipe.inputs.iter().all(|(item_id, quantity)| {
                inventory.has_item(*item_id, *quantity)
            });
            
            let has_output_space = inventory.is_full();
            
            // Only operational if we have inputs and output space
            // This assumes other systems (power, network) also set operational to false if needed
            if !has_inputs || !has_output_space {
                operational.0 = false;
            }
        }
    }
}

pub fn update_operational_status_optimized(
    mut buildings: Query<(&Position, &mut Operational, Option<&PowerConsumer>, Option<&PowerGenerator>), With<Building>>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
) {
    let has_power = power_grid.available >= 0;
   
    for (pos, mut operational, power_consumer, power_generator) in buildings.iter_mut() {
        if !network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y) {
            operational.0 = false;
            continue;
        }
       
        operational.0 = if power_generator.is_some() {
            true
        } else {
            if power_consumer.is_some() {
                has_power
            } else {
                true
            }
        }
    }
}