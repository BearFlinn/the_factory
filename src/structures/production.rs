use crate::{
    materials::{
        items::{InputPort, InventoryAccess, OutputPort, StoragePort},
        ItemName, ItemRegistry, RecipeRegistry,
    },
    structures::{Launchpad, RecipeCommitment, RecipeCrafter},
    systems::{GameScore, Operational},
    workers::tasks::{Task, TaskTarget},
};
use bevy::prelude::*;

/// Event for requesting logistics operations with port-based buildings.
/// Emitted when `OutputPort`s have items for pickup or `InputPort`s need delivery.
#[derive(Event)]
pub struct PortLogisticsRequest {
    /// The building entity that needs logistics service.
    pub building: Entity,
    /// The item type being requested or offered.
    pub item: ItemName,
    /// The quantity to transfer.
    pub quantity: u32,
    /// If true, this is an output (needs pickup). If false, this is an input (needs delivery).
    pub is_output: bool,
}

/// Timer resource for polling port logistics at regular intervals.
#[derive(Resource)]
pub struct PortLogisticsTimer {
    pub timer: Timer,
}

impl Default for PortLogisticsTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

/// Port-based crafting for buildings with both `InputPort` and `OutputPort` (e.g., Smelter, Assembler).
/// Consumes from `InputPort`, produces to `OutputPort` - cleanly separated materials.
pub fn update_port_crafters(
    mut query: Query<(
        &mut InputPort,
        &mut OutputPort,
        &mut RecipeCrafter,
        &Operational,
    )>,
    recipes: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut input_port, mut output_port, mut crafter, operational) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if !crafter.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(recipe_name) = crafter.get_active_recipe() else {
            crafter.timer.reset();
            continue;
        };

        let Some(recipe) = recipes.get_definition(recipe_name) else {
            crafter.timer.reset();
            continue;
        };

        let has_inputs = recipe
            .inputs
            .iter()
            .all(|(item, qty)| input_port.get_item_quantity(item) >= *qty);

        let has_space = output_port.has_space_for(&recipe.outputs);

        if has_inputs && has_space {
            for (item, qty) in &recipe.inputs {
                input_port.remove_item(item, *qty);
            }
            for (item, qty) in &recipe.outputs {
                output_port.add_item(item, *qty);
            }
        }

        crafter.timer.reset();
    }
}

/// Port-based crafting for Source buildings (`OutputPort` only, e.g., Mining Drill).
/// These buildings produce items without consuming any inputs.
pub fn update_source_port_crafters(
    mut query: Query<(&mut OutputPort, &mut RecipeCrafter, &Operational), Without<InputPort>>,
    recipes: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut output_port, mut crafter, operational) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if !crafter.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(recipe_name) = crafter.get_active_recipe() else {
            crafter.timer.reset();
            continue;
        };

        let Some(recipe) = recipes.get_definition(recipe_name) else {
            crafter.timer.reset();
            continue;
        };

        let has_space = output_port.has_space_for(&recipe.outputs);

        if has_space {
            for (item, qty) in &recipe.outputs {
                output_port.add_item(item, *qty);
            }
        }

        crafter.timer.reset();
    }
}

/// Port-based crafting for Sink buildings (`InputPort` only, e.g., Generator, Launchpad).
/// These buildings consume items but produce non-item outputs (like power or score).
pub fn update_sink_port_crafters(
    mut query: Query<
        (
            &mut InputPort,
            &mut RecipeCrafter,
            &Operational,
            Option<&Launchpad>,
        ),
        Without<OutputPort>,
    >,
    recipes: Res<RecipeRegistry>,
    item_registry: Res<ItemRegistry>,
    mut score: ResMut<GameScore>,
    time: Res<Time>,
) {
    for (mut input_port, mut crafter, operational, is_launchpad) in &mut query {
        if !operational.get_status() {
            continue;
        }

        if !crafter.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(recipe_name) = crafter.get_active_recipe() else {
            crafter.timer.reset();
            continue;
        };

        let Some(recipe) = recipes.get_definition(recipe_name) else {
            crafter.timer.reset();
            continue;
        };

        let has_inputs = recipe
            .inputs
            .iter()
            .all(|(item, qty)| input_port.get_item_quantity(item) >= *qty);

        if has_inputs {
            for (item, qty) in &recipe.inputs {
                input_port.remove_item(item, *qty);
            }

            if is_launchpad.is_some() {
                if let Some((item_name, _)) = recipe.inputs.iter().next() {
                    let tier = item_registry
                        .get_definition(item_name)
                        .map_or(0, |def| def.tier);
                    let points = 10 * u64::from((tier + 1).pow(2));
                    score.total_score += points;
                    score.launches_completed += 1;
                    println!(
                        "Launch completed! {} items launched for {} points (total: {})",
                        item_name, points, score.total_score
                    );
                }
            }
        }

        crafter.timer.reset();
    }
}

/// Polls port states and emits logistics requests.
/// Runs on a timer to evaluate the system state holistically.
pub fn poll_port_logistics(
    time: Res<Time>,
    mut timer: ResMut<PortLogisticsTimer>,
    // Buildings with only OutputPort (sources)
    source_ports: Query<(Entity, &OutputPort), Without<InputPort>>,
    // Buildings with only InputPort (sinks)
    sink_ports: Query<
        (
            Entity,
            &InputPort,
            Option<&RecipeCrafter>,
            Option<&RecipeCommitment>,
        ),
        Without<OutputPort>,
    >,
    // Buildings with both ports (processors)
    processor_ports: Query<(
        Entity,
        &InputPort,
        &OutputPort,
        Option<&RecipeCrafter>,
        Option<&RecipeCommitment>,
    )>,
    // Storage buildings
    storage_ports: Query<(Entity, &StoragePort)>,
    // Check existing tasks to avoid duplicates
    tasks: Query<&TaskTarget, With<Task>>,
    recipe_registry: Res<RecipeRegistry>,
    mut events: EventWriter<PortLogisticsRequest>,
) {
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }

    let existing_targets: std::collections::HashSet<Entity> =
        tasks.iter().map(|target| target.0).collect();

    for (entity, output_port) in &source_ports {
        if existing_targets.contains(&entity) {
            continue;
        }
        for (item_name, &quantity) in output_port.items() {
            if quantity > 0 {
                events.send(PortLogisticsRequest {
                    building: entity,
                    item: item_name.clone(),
                    quantity,
                    is_output: true,
                });
            }
        }
    }

    for (entity, _, output_port, _, _) in &processor_ports {
        if existing_targets.contains(&entity) {
            continue;
        }
        for (item_name, &quantity) in output_port.items() {
            if quantity > 0 {
                events.send(PortLogisticsRequest {
                    building: entity,
                    item: item_name.clone(),
                    quantity,
                    is_output: true,
                });
            }
        }
    }

    for (entity, input_port, maybe_crafter, maybe_commitment) in &sink_ports {
        if existing_targets.contains(&entity) {
            continue;
        }
        emit_input_port_requests(
            entity,
            input_port,
            maybe_crafter,
            maybe_commitment,
            &recipe_registry,
            &mut events,
        );
    }

    for (entity, input_port, _, maybe_crafter, maybe_commitment) in &processor_ports {
        if existing_targets.contains(&entity) {
            continue;
        }
        emit_input_port_requests(
            entity,
            input_port,
            maybe_crafter,
            maybe_commitment,
            &recipe_registry,
            &mut events,
        );
    }

    for (entity, storage_port) in &storage_ports {
        if existing_targets.contains(&entity) {
            continue;
        }
        for (item_name, &quantity) in storage_port.items() {
            if quantity > 0 {
                events.send(PortLogisticsRequest {
                    building: entity,
                    item: item_name.clone(),
                    quantity,
                    is_output: true,
                });
            }
        }
    }
}

/// Helper to emit delivery requests for an `InputPort` based on recipe needs.
fn emit_input_port_requests(
    entity: Entity,
    input_port: &InputPort,
    maybe_crafter: Option<&RecipeCrafter>,
    maybe_commitment: Option<&RecipeCommitment>,
    recipe_registry: &RecipeRegistry,
    events: &mut EventWriter<PortLogisticsRequest>,
) {
    let Some(crafter) = maybe_crafter else {
        return;
    };

    let recipe_name = if let Some(commitment) = maybe_commitment {
        match &commitment.committed_recipe {
            Some(recipe) => recipe,
            None => return,
        }
    } else {
        match crafter.get_active_recipe() {
            Some(recipe) => recipe,
            None => return,
        }
    };

    let Some(recipe) = recipe_registry.get_definition(recipe_name) else {
        return;
    };

    let available_space = input_port
        .capacity()
        .saturating_sub(input_port.get_total_quantity());
    if available_space == 0 {
        return;
    }

    let mut total_requested = 0u32;

    for (item_name, &recipe_quantity) in &recipe.inputs {
        let current = input_port.get_item_quantity(item_name);
        let in_transit = maybe_commitment
            .and_then(|c| c.in_transit_items.get(item_name).copied())
            .unwrap_or(0);
        let target = recipe_quantity * 10;

        let effective_amount = current.saturating_add(in_transit);
        if effective_amount < target {
            let deficit = target.saturating_sub(effective_amount);
            let feasible = deficit.min(available_space.saturating_sub(total_requested));

            if feasible > 0 {
                events.send(PortLogisticsRequest {
                    building: entity,
                    item: item_name.clone(),
                    quantity: feasible,
                    is_output: false,
                });
                total_requested += feasible;
            }
        }

        if total_requested >= available_space {
            break;
        }
    }
}
