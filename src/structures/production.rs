use crate::{
    materials::{
        items::{InputPort, InventoryAccess, OutputPort},
        ItemRegistry, RecipeRegistry,
    },
    structures::{Launchpad, RecipeCrafter},
    systems::{GameScore, Operational},
};
use bevy::prelude::*;

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
