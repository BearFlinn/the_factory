use core::fmt;

use crate::{
    grid::Position,
    materials::{InputPort, InventoryAccess, OutputPort, RecipeRegistry},
    structures::{Building, ComputeConsumer, PowerConsumer, RecipeCrafter},
    systems::{ComputeGrid, NetworkConnectivity, PowerGrid},
};
use bevy::prelude::*;

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
            OperationalCondition::HasInventorySpace(false) => write!(f, "Output full"),
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
                    let status = match condition {
                        OperationalCondition::Network(s)
                        | OperationalCondition::Power(s)
                        | OperationalCondition::Compute(s)
                        | OperationalCondition::HasItems(s)
                        | OperationalCondition::HasInventorySpace(s) => s,
                    };
                    *status
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
        Option<&InputPort>,
        Option<&OutputPort>,
    )>,
) {
    for (
        mut operational,
        building,
        power_consumer,
        compute_consumer,
        recipe_crafter,
        input_port,
        output_port,
    ) in &mut operational_query
    {
        if operational
            .0
            .as_ref()
            .is_some_and(|conditions| !conditions.is_empty())
        {
            continue;
        }

        let mut conditions = Vec::new();

        if building.is_some() {
            conditions.push(OperationalCondition::Network(false));
        }

        if power_consumer.is_some() {
            conditions.push(OperationalCondition::Power(false));
        }

        if compute_consumer.is_some() {
            conditions.push(OperationalCondition::Compute(false));
        }

        if recipe_crafter.is_some() && input_port.is_some() {
            conditions.push(OperationalCondition::HasItems(false));
        }

        if output_port.is_some() {
            conditions.push(OperationalCondition::HasInventorySpace(false));
        }

        operational.0 = Some(conditions);
    }
}

pub fn update_operational_status(
    mut operational_query: Query<(
        &mut Operational,
        Option<&RecipeCrafter>,
        Option<&InputPort>,
        Option<&OutputPort>,
        &Position,
    )>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
    compute_grid: Res<ComputeGrid>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (mut operational, crafter, input_port, output_port, pos) in &mut operational_query {
        let Some(ref mut conditions) = operational.0 else {
            continue;
        };

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
                    let Some(crafter) = crafter else {
                        continue;
                    };
                    let Some(recipe_name) = crafter.get_active_recipe() else {
                        continue;
                    };
                    let Some(recipe) = recipe_registry.get_definition(recipe_name) else {
                        continue;
                    };

                    let has_inputs = if let Some(input_port) = input_port {
                        recipe.inputs.iter().all(|(item_name, quantity)| {
                            input_port.has_at_least(item_name, *quantity)
                        })
                    } else {
                        continue;
                    };

                    *status = has_inputs;
                }

                OperationalCondition::HasInventorySpace(ref mut status) => {
                    if let Some(output_port) = output_port {
                        *status = !output_port.is_full();
                    } else {
                        *status = false;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // OperationalCondition Display trait tests
    #[test]
    fn operational_condition_network_false_displays_correctly() {
        let condition = OperationalCondition::Network(false);
        assert_eq!(format!("{condition}"), "Not connected to network");
    }

    #[test]
    fn operational_condition_power_false_displays_correctly() {
        let condition = OperationalCondition::Power(false);
        assert_eq!(format!("{condition}"), "Insufficient power");
    }

    #[test]
    fn operational_condition_compute_false_displays_correctly() {
        let condition = OperationalCondition::Compute(false);
        assert_eq!(format!("{condition}"), "Insufficient compute");
    }

    #[test]
    fn operational_condition_has_items_false_displays_correctly() {
        let condition = OperationalCondition::HasItems(false);
        assert_eq!(format!("{condition}"), "Missing required items");
    }

    #[test]
    fn operational_condition_has_inventory_space_false_displays_correctly() {
        let condition = OperationalCondition::HasInventorySpace(false);
        assert_eq!(format!("{condition}"), "Output full");
    }

    #[test]
    fn operational_condition_true_displays_empty() {
        // All true conditions should display nothing
        let conditions = [
            OperationalCondition::Network(true),
            OperationalCondition::Power(true),
            OperationalCondition::Compute(true),
            OperationalCondition::HasItems(true),
            OperationalCondition::HasInventorySpace(true),
        ];

        for condition in conditions {
            assert_eq!(format!("{condition}"), "");
        }
    }

    // Operational get_status() tests
    #[test]
    fn get_status_with_no_conditions_is_operational() {
        let operational = Operational(None);
        assert!(operational.get_status());
    }

    #[test]
    fn get_status_with_empty_conditions_is_operational() {
        let operational = Operational(Some(Vec::new()));
        assert!(operational.get_status());
    }

    #[test]
    fn get_status_with_all_conditions_true_is_operational() {
        let conditions = vec![
            OperationalCondition::Network(true),
            OperationalCondition::Power(true),
            OperationalCondition::Compute(true),
            OperationalCondition::HasItems(true),
            OperationalCondition::HasInventorySpace(true),
        ];
        let operational = Operational(Some(conditions));
        assert!(operational.get_status());
    }

    #[test]
    fn get_status_with_one_condition_false_is_not_operational() {
        // Network false
        let conditions = vec![
            OperationalCondition::Network(false),
            OperationalCondition::Power(true),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_with_power_false_is_not_operational() {
        let conditions = vec![
            OperationalCondition::Network(true),
            OperationalCondition::Power(false),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_with_compute_false_is_not_operational() {
        let conditions = vec![
            OperationalCondition::Network(true),
            OperationalCondition::Compute(false),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_with_has_items_false_is_not_operational() {
        let conditions = vec![
            OperationalCondition::Network(true),
            OperationalCondition::HasItems(false),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_with_has_inventory_space_false_is_not_operational() {
        let conditions = vec![
            OperationalCondition::Network(true),
            OperationalCondition::HasInventorySpace(false),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_with_multiple_conditions_false_is_not_operational() {
        let conditions = vec![
            OperationalCondition::Network(false),
            OperationalCondition::Power(false),
            OperationalCondition::Compute(true),
        ];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }

    #[test]
    fn get_status_single_true_condition_is_operational() {
        let conditions = vec![OperationalCondition::Network(true)];
        let operational = Operational(Some(conditions));
        assert!(operational.get_status());
    }

    #[test]
    fn get_status_single_false_condition_is_not_operational() {
        let conditions = vec![OperationalCondition::Network(false)];
        let operational = Operational(Some(conditions));
        assert!(!operational.get_status());
    }
}
