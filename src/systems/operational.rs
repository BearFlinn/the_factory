use bevy::prelude::*;
use crate::{
    grid::Position, 
    materials::{Inventory, RecipeRegistry}, 
    structures::{Building, ComputeConsumer, ComputeGenerator, PowerConsumer, PowerGenerator, RecipeCrafter}, 
    systems::{ComputeGrid, NetworkConnectivity, PowerGrid}
};

// TODO: Change Operational to an enum

#[derive(Component)]
pub struct Operational(pub bool);

pub fn update_operational_status(
    mut buildings: Query<(
        &Position,
        &mut Operational,
        Option<&PowerConsumer>,
        Option<&PowerGenerator>,
        Option<&ComputeConsumer>,
        Option<&ComputeGenerator>,
        Option<&RecipeCrafter>,
        Option<&Inventory>
    ), With<Building>>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
    compute_grid: Res<ComputeGrid>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (pos, mut operational, power_consumer, power_generator, compute_consumer, compute_generator, crafter, inventory) in buildings.iter_mut() {
        let network_ok = check_network_condition(pos, &network_connectivity);
        let power_ok = check_power_condition(power_consumer, power_generator, &power_grid);
        let compute_ok = check_compute_condition(compute_consumer, compute_generator, &compute_grid);
        let crafter_ok = check_crafter_condition(crafter, inventory, &recipe_registry);
        
        operational.0 = network_ok && power_ok && compute_ok && crafter_ok;
    }
}

fn check_network_condition(
    pos: &Position,
    network_connectivity: &NetworkConnectivity,
) -> bool {
    network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y)
}

fn check_power_condition(
    power_consumer: Option<&PowerConsumer>,
    power_generator: Option<&PowerGenerator>,
    power_grid: &PowerGrid,
) -> bool {
    let has_power = power_grid.available >= 0;
    
    if power_generator.is_some() {
        true // Generators don't need power to operate
    } else if power_consumer.is_some() {
        has_power
    } else {
        true // Buildings without power requirements are always satisfied
    }
}

fn check_compute_condition(
    compute_consumer: Option<&ComputeConsumer>,
    compute_generator: Option<&ComputeGenerator>,
    compute_grid: &ComputeGrid,
) -> bool {
    let has_compute = compute_grid.available >= 0;
    
    if compute_generator.is_some() {
        true // Generators don't need compute to operate
    } else if compute_consumer.is_some() {
        has_compute
    } else {
        true // Buildings without compute requirements are always satisfied
    }
}

fn check_crafter_condition(
    crafter: Option<&RecipeCrafter>,
    inventory: Option<&Inventory>,
    recipe_registry: &RecipeRegistry,
) -> bool {
    if let (Some(crafter), Some(inventory)) = (crafter, inventory) {
        if let Some(recipe) = recipe_registry.get_definition(&crafter.recipe) {
            let has_inputs = recipe.inputs.iter().all(|(item_name, quantity)| {
                inventory.has_at_least(item_name, *quantity)
            });
            
            let has_output_space = !inventory.is_full();
            
            return has_inputs && has_output_space;
        }
    }
    true // Buildings without crafters are always satisfied
}