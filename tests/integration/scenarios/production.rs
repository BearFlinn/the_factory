use bevy::prelude::*;
use the_factory::{
    materials::{InputPort, InventoryAccess, OutputPort},
    structures::RecipeCrafter,
};

use crate::harness::*;

#[test]
fn operational_crafter_produces_output() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0), (3, 0)]);

    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let smelter = spawn_building(&mut app, "Smelter", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        let mut crafter = world.get_mut::<RecipeCrafter>(smelter).unwrap();
        crafter.current_recipe = Some("Iron Ingot".to_string());
    }

    {
        let world = app.world_mut();
        add_items_to_input(world, smelter, "Iron Ore", 10);
        add_items_to_input(world, smelter, "Coal", 5);
    }

    tick_seconds(&mut app, 0.5);
    tick_n(&mut app, 5);

    tick_seconds(&mut app, 2.5);
    tick_n(&mut app, 5);

    let output_port = app.world().get::<OutputPort>(smelter).unwrap();
    assert!(
        output_port.get_item_quantity("Iron Ingot") > 0,
        "smelter should have produced Iron Ingot, output: {:?}",
        output_port.items
    );
}

#[test]
fn non_operational_crafter_halts() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(20, 20)]);

    let smelter = spawn_building(&mut app, "Smelter", 20, 20);

    {
        let world = app.world_mut();
        let mut crafter = world.get_mut::<RecipeCrafter>(smelter).unwrap();
        crafter.current_recipe = Some("Iron Ingot".to_string());
    }
    {
        let world = app.world_mut();
        add_items_to_input(world, smelter, "Iron Ore", 10);
        add_items_to_input(world, smelter, "Coal", 5);
    }

    tick_seconds(&mut app, 5.0);
    tick_n(&mut app, 10);

    let output = app.world().get::<OutputPort>(smelter).unwrap();
    assert!(
        output.is_empty(),
        "non-operational smelter should not produce anything, output: {:?}",
        output.items
    );
    assert_not_operational(app.world(), smelter);
}

#[test]
fn source_crafter_produces_without_inputs() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);
    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(3, 0)]);
    let generator = spawn_building(&mut app, "Generator", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        add_items_to_input(world, generator, "Coal", 10);
    }

    tick_seconds(&mut app, 0.5);
    tick_n(&mut app, 5);

    tick_seconds(&mut app, 3.5);
    tick_n(&mut app, 5);

    let input = app.world().get::<InputPort>(generator).unwrap();
    assert!(
        input.get_item_quantity("Coal") < 10,
        "generator should have consumed some Coal, but still has {}",
        input.get_item_quantity("Coal")
    );
}

#[test]
fn crafter_waits_for_inputs() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);
    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(3, 0)]);
    let smelter = spawn_building(&mut app, "Smelter", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        let mut crafter = world.get_mut::<RecipeCrafter>(smelter).unwrap();
        crafter.current_recipe = Some("Iron Ingot".to_string());
    }

    tick_seconds(&mut app, 5.0);
    tick_n(&mut app, 10);

    let output = app.world().get::<OutputPort>(smelter).unwrap();
    assert!(
        output.is_empty(),
        "smelter with no inputs should not produce anything, output: {:?}",
        output.items
    );
}

#[test]
fn input_limits_match_recipe() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);
    let _connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(3, 0)]);
    let smelter = spawn_building(&mut app, "Smelter", 3, 0);
    tick_n(&mut app, 3);

    {
        let world = app.world_mut();
        let mut crafter = world.get_mut::<RecipeCrafter>(smelter).unwrap();
        crafter.current_recipe = Some("Iron Ingot".to_string());
    }

    tick_n(&mut app, 3);

    let input = app.world().get::<InputPort>(smelter).unwrap();
    assert_eq!(
        input.item_limits.get("Iron Ore").copied().unwrap_or(0),
        34,
        "Iron Ore limit should be 34, got {:?}",
        input.item_limits
    );
    assert_eq!(
        input.item_limits.get("Coal").copied().unwrap_or(0),
        17,
        "Coal limit should be 17, got {:?}",
        input.item_limits
    );
}
