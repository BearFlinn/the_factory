use core::fmt;

use bevy::prelude::*;
use crate::{
    grid::Position, 
    materials::{Inventory, InventoryType, InventoryTypes, RecipeRegistry}, 
    structures::{Building, ComputeConsumer, PowerConsumer, RecipeCrafter}, 
    systems::{ComputeGrid, NetworkConnectivity, PowerGrid}
};

#[derive(Debug)]
pub enum OperationalCondition {
    Network(bool),
    Power(bool),
    Compute(bool),
    HasItems(bool),
    HasInventorySpace(bool),
}

impl fmt::Display for OperationalCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationalCondition::Network(false) => write!(f, "Not connected to network"),
            OperationalCondition::Power(false) => write!(f, "Insufficient power"),
            OperationalCondition::Compute(false) => write!(f, "Insufficient compute"),
            OperationalCondition::HasItems(false) => write!(f, "Missing required items"),
            OperationalCondition::HasInventorySpace(false) => write!(f, "Inventory full"),
            _ => Ok(()),
        }
    }
}

#[derive(Component, Debug)]
pub struct Operational(pub Option<Vec<OperationalCondition>>);

impl Operational {
    pub fn get_status(&self) -> bool {
        match &self.0 {
            None => true, // No conditions means operational
            Some(conditions) => {
                // All conditions must be true for operational status
                conditions.iter().all(|condition| {
                    match condition {
                        OperationalCondition::Network(status) => *status,
                        OperationalCondition::Power(status) => *status,
                        OperationalCondition::Compute(status) => *status,
                        OperationalCondition::HasItems(status) => *status,
                        OperationalCondition::HasInventorySpace(status) => *status,
                    }
                })
            }
        }
    }
}

pub fn populate_operational_conditions(
    mut operational_query: Query<(
        &mut Operational,
        Option<&Building>,
        Option<&PowerConsumer>,
        Option<&ComputeConsumer>,
        Option<&RecipeCrafter>,
        Option<&Inventory>,
        Option<&InventoryType>,
    )>,
) {
    for (mut operational, building, power_consumer, compute_consumer, recipe_crafter, inventory, inventory_type) in operational_query.iter_mut() {
        // Only populate if conditions are None or empty
        if operational.0.is_some() && !operational.0.as_ref().unwrap().is_empty() {
            continue;
        }

        let mut conditions = Vec::new();

        // Always add Network condition for buildings
        if building.is_some() {
            conditions.push(OperationalCondition::Network(false));
        }

        // Add Power condition if entity consumes power
        if power_consumer.is_some() {
            conditions.push(OperationalCondition::Power(false));
        }

        // Add Compute condition if entity consumes compute
        if compute_consumer.is_some() {
            conditions.push(OperationalCondition::Compute(false));
        }

        // Add HasItems condition if entity crafts recipes (needs input materials)
        if recipe_crafter.is_some() {
            conditions.push(OperationalCondition::HasItems(false));
        }

        // Add HasInventorySpace condition if entity has inventory and produces/sends items
        if let (Some(_inventory), Some(inv_type)) = (inventory, inventory_type) {
            match inv_type.0 {
                InventoryTypes::Sender | InventoryTypes::Producer => {
                    conditions.push(OperationalCondition::HasInventorySpace(false));
                }
                _ => {} // Storage, Requester, Carrier don't need space checks for operation
            }
        }

        // Set the populated conditions
        operational.0 = Some(conditions);
    }
}

pub fn update_operational_status(
    mut operational_query: Query<(
        &mut Operational,
        Option<&RecipeCrafter>,
        Option<&Inventory>,
        &Position,
    )>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
    compute_grid: Res<ComputeGrid>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (mut operational, crafter, inventory, pos) in operational_query.iter_mut() {
        
        // Skip entities without operational conditions
        let Some(ref mut conditions) = operational.0 else {
            continue;
        };

        // Iterate through conditions and update each based on type
        for condition in conditions.iter_mut() {
            match condition {
                OperationalCondition::Network(ref mut status) => {
                    *status = network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y);
                }
                
                OperationalCondition::Power(ref mut status) => {
                    *status = power_grid.available >= 0;
                }
                
                OperationalCondition::Compute(ref mut status) => {
                    *status = compute_grid.available >= 0;
                }
                
                OperationalCondition::HasItems(ref mut status) => {
                    if let (Some(crafter), Some(inventory)) = (crafter, inventory) {
                        let Some(recipe_name) = crafter.get_active_recipe() else {
                            continue;
                        };
                        if let Some(recipe) = recipe_registry.get_definition(recipe_name) {
                            let has_inputs = recipe.inputs.iter().all(|(item_name, quantity)| {
                                inventory.has_at_least(item_name, *quantity)
                            });
                            
                            *status = has_inputs;
                        }
                    }
                }
                
                OperationalCondition::HasInventorySpace(ref mut status) => {
                    if let Some(inventory) = inventory {
                        *status = !inventory.is_full();
                    } else {
                        *status = false;
                    }
                }
            }
        }
    }
}