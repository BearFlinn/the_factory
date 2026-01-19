use std::collections::HashMap;

use crate::{
    grid::Position,
    materials::{items::Inventory, InventoryType, InventoryTypes, ItemName, RecipeRegistry},
    structures::RecipeCrafter,
    systems::Operational,
    workers::tasks::{Priority, Task, TaskTarget},
};
use bevy::prelude::*;

#[allow(clippy::needless_pass_by_value)]
pub fn update_recipe_crafters(
    mut query: Query<(&mut RecipeCrafter, &Operational, &mut Inventory)>,
    recipe_registry: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut crafter, operational, mut inventory) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if crafter.timer.tick(time.delta()).just_finished() {
            let Some(recipe_name) = crafter.get_active_recipe() else {
                continue;
            };
            if let Some(recipe) = recipe_registry.get_definition(recipe_name) {
                // Check if we have all required inputs
                let can_craft = !inventory.is_full()
                    || recipe
                        .inputs
                        .iter()
                        .all(|(item_name, quantity)| inventory.has_at_least(item_name, *quantity));

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
    pub priority: Priority,
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn crafter_logistics_requests(
    mut crafters: Query<
        (
            Entity,
            &mut RecipeCrafter,
            &Inventory,
            &InventoryType,
            &Position,
        ),
        Changed<Inventory>,
    >,
    tasks: Query<(Entity, &TaskTarget, &Priority), With<Task>>,
    mut events: EventWriter<CrafterLogisticsRequest>,
    recipe_registry: Res<RecipeRegistry>,
) {
    const WORKER_CAPACITY: u32 = 20;

    for (crafter_entity, crafter, inventory, inv_type, position) in &mut crafters {
        let existing_priorities: std::collections::HashSet<_> = tasks
            .iter()
            .filter(|(_, target_entity, _)| target_entity.0 == crafter_entity)
            .map(|(_, _, priority)| priority)
            .collect();

        match inv_type.0 {
            InventoryTypes::Sender => {
                let total_items = inventory.get_total_quantity();

                if total_items >= WORKER_CAPACITY
                    && !existing_priorities.contains(&Priority::Medium)
                {
                    events.send(CrafterLogisticsRequest {
                        crafter: crafter_entity,
                        position: *position,
                        needs: None,
                        has: Some(inventory.get_all_items()),
                        priority: Priority::Medium,
                    });
                }
            }
            InventoryTypes::Requester => {
                // Only process logistics if a recipe is selected
                if let Some(recipe_name) = crafter.get_active_recipe() {
                    if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                        let required_items: HashMap<_, _> = recipe_def
                            .inputs
                            .iter()
                            .map(|(item, quantity)| (item.clone(), quantity * 10))
                            .collect();

                        if !inventory.has_items_for_recipe(&required_items)
                            && !existing_priorities.contains(&Priority::Medium)
                        {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: *position,
                                needs: Some(required_items),
                                has: None,
                                priority: Priority::Medium,
                            });
                        }
                    }
                }
            }
            InventoryTypes::Producer => {
                // Only process logistics if a recipe is selected
                if let Some(recipe_name) = crafter.get_active_recipe() {
                    if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                        // Calculate what we actually need (smart requesting)
                        let mut needed_items = HashMap::new();
                        let mut total_needed = 0u32;

                        for (item_name, &recipe_quantity) in &recipe_def.inputs {
                            let current_quantity = inventory.get_item_quantity(item_name);
                            let target_quantity = recipe_quantity * 10; // Desired buffer

                            if current_quantity < target_quantity {
                                let needed = target_quantity - current_quantity;
                                // Respect inventory capacity limits
                                let available_space = inventory
                                    .capacity
                                    .saturating_sub(inventory.get_total_quantity());
                                let feasible_amount =
                                    needed.min(available_space.saturating_sub(total_needed));

                                if feasible_amount > 0 {
                                    needed_items.insert(item_name.clone(), feasible_amount);
                                    total_needed += feasible_amount;
                                }
                            }

                            // Stop if we've filled the available space
                            if total_needed
                                >= inventory
                                    .capacity
                                    .saturating_sub(inventory.get_total_quantity())
                            {
                                break;
                            }
                        }

                        // Only request if we actually need something and don't have existing tasks
                        if !needed_items.is_empty()
                            && !existing_priorities.contains(&Priority::Medium)
                        {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: *position,
                                needs: Some(needed_items),
                                has: None,
                                priority: Priority::Medium,
                            });
                        }

                        // Handle output removal (unchanged)
                        let produced_items = inventory.recipe_output_amounts(&recipe_def.outputs);

                        if produced_items.values().sum::<u32>() >= WORKER_CAPACITY
                            && !existing_priorities.contains(&Priority::Medium)
                        {
                            events.send(CrafterLogisticsRequest {
                                crafter: crafter_entity,
                                position: *position,
                                needs: None,
                                has: Some(produced_items),
                                priority: Priority::Medium,
                            });
                        }
                    }
                }
            }
            InventoryTypes::Storage | InventoryTypes::Carrier => {}
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn handle_recipe_selection_logistics(
    mut crafters: Query<
        (
            Entity,
            &RecipeCrafter,
            &Inventory,
            &InventoryType,
            &Position,
        ),
        Changed<RecipeCrafter>,
    >,
    tasks: Query<(Entity, &TaskTarget, &Priority), With<Task>>,
    mut events: EventWriter<CrafterLogisticsRequest>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (crafter_entity, crafter, inventory, inv_type, position) in &mut crafters {
        // Only process if a recipe was just selected (not cleared)
        let Some(recipe_name) = crafter.get_active_recipe() else {
            continue;
        };

        // Only handle requesters and producers that need materials
        if !matches!(
            inv_type.0,
            InventoryTypes::Requester | InventoryTypes::Producer
        ) {
            continue;
        }

        // Check if there are already active tasks for this crafter
        let has_existing_tasks = tasks
            .iter()
            .any(|(_, target, _)| target.0 == crafter_entity);
        if has_existing_tasks {
            continue;
        }

        // Get recipe definition and check if materials are needed
        if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
            let required_items: HashMap<_, _> = recipe_def
                .inputs
                .iter()
                .map(|(item, quantity)| (item.clone(), quantity * 10))
                .collect();

            // Only request logistics if we actually need materials
            if !inventory.has_items_for_recipe(&required_items) {
                events.send(CrafterLogisticsRequest {
                    crafter: crafter_entity,
                    position: *position,
                    needs: Some(required_items),
                    has: None,
                    priority: Priority::Medium,
                });

                println!(
                    "Recipe selection logistics: Requesting materials for {} at ({}, {})",
                    recipe_name, position.x, position.y
                );
            }
        }
    }
}
