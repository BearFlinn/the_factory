use std::collections::HashMap;

use crate::{
    grid::Position,
    materials::{
        items::{InputBuffer, Inventory, OutputBuffer},
        InventoryType, InventoryTypes, ItemName, RecipeRegistry,
    },
    structures::{Processor, RecipeCrafter, Sink, Source},
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

/// Buffer-aware crafting for Processor buildings (input + output buffers).
/// Consumes from `InputBuffer`, produces to `OutputBuffer` - no mixing of materials.
#[allow(clippy::needless_pass_by_value)]
pub fn update_processor_crafters(
    mut query: Query<
        (
            &mut RecipeCrafter,
            &Operational,
            &mut InputBuffer,
            &mut OutputBuffer,
        ),
        With<Processor>,
    >,
    recipe_registry: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut crafter, operational, mut input_buffer, mut output_buffer) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if crafter.timer.tick(time.delta()).just_finished() {
            let Some(recipe_name) = crafter.get_active_recipe() else {
                continue;
            };
            let Some(recipe) = recipe_registry.get_definition(recipe_name) else {
                continue;
            };

            // Check if we have all required inputs in InputBuffer
            let has_inputs = input_buffer.inventory.has_items_for_recipe(&recipe.inputs);

            // Check if we have space for outputs in OutputBuffer
            let has_output_space = output_buffer.inventory.has_space_for(&recipe.outputs);

            if has_inputs && has_output_space {
                // Consume inputs from InputBuffer
                for (item_name, quantity) in &recipe.inputs {
                    input_buffer.inventory.remove_item(item_name, *quantity);
                }

                // Produce outputs to OutputBuffer
                for (item_name, quantity) in &recipe.outputs {
                    output_buffer.inventory.add_item(item_name, *quantity);
                }
            }

            crafter.timer.reset();
        }
    }
}

/// Buffer-aware crafting for Source buildings (output buffer only, no inputs).
/// Mining drills and similar buildings that produce items from nothing (or from the world).
#[allow(clippy::needless_pass_by_value)]
pub fn update_source_crafters(
    mut query: Query<(&mut RecipeCrafter, &Operational, &mut OutputBuffer), With<Source>>,
    recipe_registry: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut crafter, operational, mut output_buffer) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if crafter.timer.tick(time.delta()).just_finished() {
            let Some(recipe_name) = crafter.get_active_recipe() else {
                continue;
            };
            let Some(recipe) = recipe_registry.get_definition(recipe_name) else {
                continue;
            };

            // Sources don't consume inputs - check if we have space for outputs
            let has_output_space = output_buffer.inventory.has_space_for(&recipe.outputs);

            if has_output_space {
                // Produce outputs to OutputBuffer
                for (item_name, quantity) in &recipe.outputs {
                    output_buffer.inventory.add_item(item_name, *quantity);
                }
            }

            crafter.timer.reset();
        }
    }
}

/// Buffer-aware crafting for Sink buildings (input buffer only, consumes for non-item output).
/// Generators and similar buildings that consume items but don't produce items.
#[allow(clippy::needless_pass_by_value)]
pub fn update_sink_crafters(
    mut query: Query<(&mut RecipeCrafter, &Operational, &mut InputBuffer), With<Sink>>,
    recipe_registry: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut crafter, operational, mut input_buffer) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if crafter.timer.tick(time.delta()).just_finished() {
            let Some(recipe_name) = crafter.get_active_recipe() else {
                continue;
            };
            let Some(recipe) = recipe_registry.get_definition(recipe_name) else {
                continue;
            };

            // Check if we have all required inputs in InputBuffer
            let has_inputs = input_buffer.inventory.has_items_for_recipe(&recipe.inputs);

            if has_inputs {
                // Consume inputs from InputBuffer (outputs are non-item effects like power)
                for (item_name, quantity) in &recipe.inputs {
                    input_buffer.inventory.remove_item(item_name, *quantity);
                }
                // Note: Outputs like power generation are handled by other systems
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

// ============================================================================
// Polling-Based Buffer Logistics System
// ============================================================================
// Replaces the reactive Changed<Inventory> approach with declarative polling.
// This evaluates buffer states holistically and avoids race conditions.

/// Event for requesting logistics operations between buffer-based buildings.
/// Unlike `CrafterLogisticsRequest`, this is designed for the new buffer system.
#[derive(Event)]
pub struct BufferLogisticsRequest {
    /// Entity that needs items (has `InputBuffer` below threshold) or `None` for offers.
    pub requester: Option<Entity>,
    /// Entity that has items to offer (has `OutputBuffer` above threshold) or `None` for requests.
    pub offerer: Option<Entity>,
    /// Position for pathfinding (requester position for requests, offerer for offers).
    pub position: Position,
    /// Items being requested or offered.
    pub items: HashMap<ItemName, u32>,
    /// Task priority.
    pub priority: Priority,
}

/// Timer resource for polling buffer logistics at regular intervals.
#[derive(Resource)]
pub struct LogisticsPlannerTimer {
    pub timer: Timer,
}

impl Default for LogisticsPlannerTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

/// Polls buffer states and emits logistics requests.
/// Runs on a timer rather than reactively to avoid race conditions and evaluate
/// the system state holistically.
// Multiple queries are needed to handle different building archetypes and check existing tasks.
#[allow(
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::too_many_arguments
)]
pub fn poll_buffer_logistics(
    time: Res<Time>,
    mut timer: ResMut<LogisticsPlannerTimer>,
    // Query for buildings with OutputBuffers that might have items to offer
    output_buffers: Query<(Entity, &Position, &OutputBuffer), Without<InputBuffer>>,
    // Query for buildings with InputBuffers that might need items
    input_buffers: Query<(Entity, &Position, &InputBuffer, Option<&RecipeCrafter>)>,
    // Query for Processor buildings (have both buffers)
    processors: Query<
        (
            Entity,
            &Position,
            &InputBuffer,
            &OutputBuffer,
            Option<&RecipeCrafter>,
        ),
        With<Processor>,
    >,
    // Check existing tasks to avoid duplicates
    tasks: Query<&TaskTarget, With<Task>>,
    recipe_registry: Res<RecipeRegistry>,
    mut events: EventWriter<BufferLogisticsRequest>,
) {
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }

    let existing_targets: std::collections::HashSet<Entity> =
        tasks.iter().map(|target| target.0).collect();

    // Collect offers from OutputBuffers above threshold
    let mut offers: Vec<(Entity, Position, HashMap<ItemName, u32>)> = Vec::new();

    // Source buildings (output only)
    for (entity, position, output_buffer) in &output_buffers {
        if existing_targets.contains(&entity) {
            continue;
        }
        if output_buffer.has_items_to_offer() {
            let items = output_buffer.inventory.get_all_items();
            if !items.is_empty() {
                offers.push((entity, *position, items));
            }
        }
    }

    // Processor buildings (output buffer part)
    for (entity, position, _, output_buffer, _) in &processors {
        if existing_targets.contains(&entity) {
            continue;
        }
        if output_buffer.has_items_to_offer() {
            let items = output_buffer.inventory.get_all_items();
            if !items.is_empty() {
                offers.push((entity, *position, items));
            }
        }
    }

    // Collect requests from InputBuffers below threshold
    let mut requests: Vec<(Entity, Position, HashMap<ItemName, u32>)> = Vec::new();

    // Sink buildings (input only) and other input-buffer buildings
    for (entity, position, input_buffer, maybe_crafter) in &input_buffers {
        if existing_targets.contains(&entity) {
            continue;
        }
        if !input_buffer.needs_items() {
            continue;
        }

        // If building has a recipe, request recipe inputs
        if let Some(crafter) = maybe_crafter {
            if let Some(recipe_name) = crafter.get_active_recipe() {
                if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                    let needed = calculate_buffer_needs(input_buffer, &recipe_def.inputs);
                    if !needed.is_empty() {
                        requests.push((entity, *position, needed));
                    }
                }
            }
        }
    }

    // Processor buildings (input buffer part)
    for (entity, position, input_buffer, _, maybe_crafter) in &processors {
        if existing_targets.contains(&entity) {
            continue;
        }
        if !input_buffer.needs_items() {
            continue;
        }

        if let Some(crafter) = maybe_crafter {
            if let Some(recipe_name) = crafter.get_active_recipe() {
                if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                    let needed = calculate_buffer_needs(input_buffer, &recipe_def.inputs);
                    if !needed.is_empty() {
                        requests.push((entity, *position, needed));
                    }
                }
            }
        }
    }

    // Emit offer events (items available for pickup)
    for (entity, position, items) in offers {
        events.send(BufferLogisticsRequest {
            requester: None,
            offerer: Some(entity),
            position,
            items,
            priority: Priority::Medium,
        });
    }

    // Emit request events (items needed for delivery)
    for (entity, position, items) in requests {
        events.send(BufferLogisticsRequest {
            requester: Some(entity),
            offerer: None,
            position,
            items,
            priority: Priority::Medium,
        });
    }
}

/// Calculate what items a buffer needs based on recipe inputs and current inventory.
fn calculate_buffer_needs(
    input_buffer: &InputBuffer,
    recipe_inputs: &HashMap<ItemName, u32>,
) -> HashMap<ItemName, u32> {
    let mut needed = HashMap::new();
    let available_space = input_buffer
        .inventory
        .capacity
        .saturating_sub(input_buffer.inventory.get_total_quantity());

    if available_space == 0 {
        return needed;
    }

    let mut total_needed = 0u32;

    for (item_name, &recipe_quantity) in recipe_inputs {
        let current = input_buffer.inventory.get_item_quantity(item_name);
        let target = recipe_quantity * 10; // Buffer 10x recipe amount

        if current < target {
            let deficit = target - current;
            let feasible = deficit.min(available_space.saturating_sub(total_needed));

            if feasible > 0 {
                needed.insert(item_name.clone(), feasible);
                total_needed += feasible;
            }
        }

        if total_needed >= available_space {
            break;
        }
    }

    needed
}
