use crate::{
    materials::{
        items::{InputPort, InventoryAccess, ItemName, OutputPort},
        ItemRegistry, RecipeRegistry,
    },
    structures::{ConstructionSite, Launchpad, RecipeCrafter},
    systems::{GameScore, Operational},
};
use bevy::prelude::*;
use std::collections::HashMap;

pub fn compute_item_limits(
    capacity: u32,
    recipe_inputs: &HashMap<ItemName, u32>,
) -> HashMap<ItemName, u32> {
    if recipe_inputs.is_empty() {
        return HashMap::new();
    }

    let total_recipe_qty: u32 = recipe_inputs.values().sum();

    recipe_inputs
        .iter()
        .map(|(item, &qty)| {
            let proportional = u32::try_from(
                (u64::from(capacity) * u64::from(qty)).div_ceil(u64::from(total_recipe_qty)),
            )
            .unwrap_or(u32::MAX);
            let limit = proportional.max(qty);
            (item.clone(), limit)
        })
        .collect()
}

pub fn sync_input_port_limits(
    mut query: Query<(&mut InputPort, &RecipeCrafter), Without<ConstructionSite>>,
    recipes: Res<RecipeRegistry>,
) {
    for (mut input_port, crafter) in &mut query {
        let new_limits = crafter
            .get_active_recipe()
            .and_then(|name| recipes.get_definition(name))
            .map_or_else(HashMap::new, |recipe| {
                compute_item_limits(input_port.capacity, &recipe.inputs)
            });

        if input_port.item_limits != new_limits {
            input_port.item_limits = new_limits;
        }
    }
}

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::materials::RecipeName;
    use bevy::ecs::system::SystemState;

    fn make_recipe_registry(ron: &str) -> RecipeRegistry {
        RecipeRegistry::from_ron(ron).unwrap()
    }

    #[test]
    fn compute_item_limits_multi_input_proportional() {
        let mut inputs = HashMap::new();
        inputs.insert("Iron Ore".to_string(), 2);
        inputs.insert("Coal".to_string(), 1);

        let limits = compute_item_limits(50, &inputs);

        // ceil(50 * 2/3) = 34, max(34, 2) = 34
        assert_eq!(limits.get("Iron Ore").copied().unwrap(), 34);
        // ceil(50 * 1/3) = 17, max(17, 1) = 17
        assert_eq!(limits.get("Coal").copied().unwrap(), 17);
        assert_eq!(limits.len(), 2);
    }

    #[test]
    fn compute_item_limits_single_input_gets_full_capacity() {
        let mut inputs = HashMap::new();
        inputs.insert("Coal".to_string(), 1);

        let limits = compute_item_limits(50, &inputs);

        assert_eq!(limits.get("Coal").copied().unwrap(), 50);
        assert_eq!(limits.len(), 1);
    }

    #[test]
    fn compute_item_limits_empty_inputs_returns_empty() {
        let inputs = HashMap::new();
        let limits = compute_item_limits(50, &inputs);
        assert!(limits.is_empty());
    }

    #[test]
    fn compute_item_limits_guarantees_at_least_one_batch() {
        let mut inputs = HashMap::new();
        inputs.insert("Rare".to_string(), 10);
        inputs.insert("Common".to_string(), 1);

        // capacity=5, Rare: ceil(5*10/11)=5, max(5,10)=10
        let limits = compute_item_limits(5, &inputs);
        assert!(limits.get("Rare").copied().unwrap() >= 10);
        assert!(limits.get("Common").copied().unwrap() >= 1);
    }

    #[test]
    fn compute_item_limits_equal_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("A".to_string(), 1);
        inputs.insert("B".to_string(), 1);

        let limits = compute_item_limits(100, &inputs);

        assert_eq!(limits.get("A").copied().unwrap(), 50);
        assert_eq!(limits.get("B").copied().unwrap(), 50);
    }

    #[test]
    fn sync_sets_limits_from_active_recipe() {
        let mut app = App::new();

        let ron = r#"[
            (
                name: "Iron Ingot",
                inputs: {"Iron Ore": 2, "Coal": 1},
                outputs: {"Iron Ingot": 1},
                crafting_time: 2.0,
            ),
        ]"#;
        let registry = make_recipe_registry(ron);
        app.insert_resource(registry);

        let recipe_name: RecipeName = "Iron Ingot".to_string();
        let crafter = RecipeCrafter {
            current_recipe: Some(recipe_name),
            available_recipes: Vec::new(),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let entity = app.world_mut().spawn((InputPort::new(50), crafter)).id();

        let mut system_state: SystemState<(
            Query<(&mut InputPort, &RecipeCrafter), Without<ConstructionSite>>,
            Res<RecipeRegistry>,
        )> = SystemState::new(app.world_mut());

        let (query, recipes) = system_state.get_mut(app.world_mut());
        sync_input_port_limits(query, recipes);
        system_state.apply(app.world_mut());

        let port = app.world().entity(entity).get::<InputPort>().unwrap();
        assert_eq!(port.item_limits.get("Iron Ore").copied().unwrap(), 34);
        assert_eq!(port.item_limits.get("Coal").copied().unwrap(), 17);
        assert_eq!(port.item_limits.len(), 2);
    }

    #[test]
    fn sync_clears_limits_when_no_recipe() {
        let mut app = App::new();

        let ron = "[]";
        let registry = make_recipe_registry(ron);
        app.insert_resource(registry);

        let crafter = RecipeCrafter {
            current_recipe: None,
            available_recipes: Vec::new(),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let mut port = InputPort::new(50);
        port.item_limits.insert("Stale".to_string(), 25);

        let entity = app.world_mut().spawn((port, crafter)).id();

        let mut system_state: SystemState<(
            Query<(&mut InputPort, &RecipeCrafter), Without<ConstructionSite>>,
            Res<RecipeRegistry>,
        )> = SystemState::new(app.world_mut());

        let (query, recipes) = system_state.get_mut(app.world_mut());
        sync_input_port_limits(query, recipes);
        system_state.apply(app.world_mut());

        let port = app.world().entity(entity).get::<InputPort>().unwrap();
        assert!(port.item_limits.is_empty());
    }

    #[test]
    fn sync_skips_construction_sites() {
        let mut app = App::new();

        let ron = r#"[
            (
                name: "Test",
                inputs: {"A": 1},
                outputs: {"B": 1},
                crafting_time: 1.0,
            ),
        ]"#;
        let registry = make_recipe_registry(ron);
        app.insert_resource(registry);

        let crafter = RecipeCrafter {
            current_recipe: Some("Test".to_string()),
            available_recipes: Vec::new(),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let entity = app
            .world_mut()
            .spawn((
                InputPort::new(50),
                crafter,
                ConstructionSite {
                    building_name: "Smelter".to_string(),
                },
            ))
            .id();

        let mut system_state: SystemState<(
            Query<(&mut InputPort, &RecipeCrafter), Without<ConstructionSite>>,
            Res<RecipeRegistry>,
        )> = SystemState::new(app.world_mut());

        let (query, recipes) = system_state.get_mut(app.world_mut());
        sync_input_port_limits(query, recipes);
        system_state.apply(app.world_mut());

        let port = app.world().entity(entity).get::<InputPort>().unwrap();
        assert!(port.item_limits.is_empty());
    }
}
