use bevy::prelude::*;
use the_factory::{
    grid::Position,
    materials::{InputPort, InventoryAccess},
    structures::{Building, ConstructionSite},
    systems::Operational,
};

use crate::harness::*;

#[test]
fn placement_creates_construction_site() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    let mut found = false;
    for (_, site, pos) in app
        .world_mut()
        .query::<(Entity, &ConstructionSite, &Position)>()
        .iter(app.world())
    {
        if pos.x == 2 && pos.y == 0 {
            assert_eq!(site.building_name, "Connector");
            found = true;
        }
    }
    assert!(found, "should have created a ConstructionSite at (2,0)");
}

#[test]
fn construction_completes_with_materials() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    let site_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Position), With<ConstructionSite>>();
        let mut found = None;
        for (entity, pos) in query.iter(app.world()) {
            if pos.x == 2 && pos.y == 0 {
                found = Some(entity);
            }
        }
        found.expect("construction site should exist at (2,0)")
    };

    {
        let world = app.world_mut();
        add_items_to_input(world, site_entity, "Iron Ore", 10);
        add_items_to_input(world, site_entity, "Copper Ore", 5);
    }

    tick_n(&mut app, 5);

    assert!(
        app.world().get::<ConstructionSite>(site_entity).is_none(),
        "construction site should have been consumed"
    );

    let mut building_found = false;
    {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Position), With<Building>>();
        for (_, pos) in query.iter(app.world()) {
            if pos.x == 2 && pos.y == 0 {
                building_found = true;
            }
        }
    }
    assert!(building_found, "completed building should exist at (2,0)");
}

#[test]
fn completed_building_has_all_components() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    let site_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Position), With<ConstructionSite>>();
        let mut found = None;
        for (entity, pos) in query.iter(app.world()) {
            if pos.x == 2 && pos.y == 0 {
                found = Some(entity);
            }
        }
        found.expect("construction site should exist")
    };

    {
        let world = app.world_mut();
        add_items_to_input(world, site_entity, "Iron Ore", 10);
        add_items_to_input(world, site_entity, "Copper Ore", 5);
    }
    tick_n(&mut app, 5);

    let building_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Position), With<Building>>();
        let mut found = None;
        for (entity, pos) in query.iter(app.world()) {
            if pos.x == 2 && pos.y == 0 {
                found = Some(entity);
            }
        }
        found.expect("completed building should exist at (2,0)")
    };

    assert_has_component::<Building>(app.world(), building_entity);
    assert_has_component::<Name>(app.world(), building_entity);
    assert_has_component::<Operational>(app.world(), building_entity);
    assert_has_component::<Position>(app.world(), building_entity);
    assert_has_component::<Transform>(app.world(), building_entity);
}

#[test]
fn placement_rejected_on_occupied_cell() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    let count_before = {
        let mut query = app
            .world_mut()
            .query_filtered::<&Position, With<ConstructionSite>>();
        query
            .iter(app.world())
            .filter(|p| p.x == 2 && p.y == 0)
            .count()
    };
    assert_eq!(count_before, 1, "should have exactly one construction site");

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    let count_after = {
        let mut query = app
            .world_mut()
            .query_filtered::<&Position, With<ConstructionSite>>();
        query
            .iter(app.world())
            .filter(|p| p.x == 2 && p.y == 0)
            .count()
    };
    assert_eq!(
        count_after, 1,
        "should still have exactly one construction site after rejected placement"
    );
}

#[test]
fn auto_pull_delivers_materials() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    app.world_mut()
        .write_message(the_factory::structures::PlaceBuildingRequestEvent {
            building_name: "Connector".to_string(),
            grid_x: 2,
            grid_y: 0,
        });
    tick_n(&mut app, 3);

    tick_seconds(&mut app, 1.5);
    tick_n(&mut app, 10);

    let site_entity = {
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &Position), With<ConstructionSite>>();
        let mut found = None;
        for (entity, pos) in query.iter(app.world()) {
            if pos.x == 2 && pos.y == 0 {
                found = Some(entity);
            }
        }
        found
    };

    if let Some(site_entity) = site_entity {
        let input = app.world().get::<InputPort>(site_entity).unwrap();
        let iron = input.get_item_quantity("Iron Ore");
        let copper = input.get_item_quantity("Copper Ore");
        assert!(
            iron > 0 || copper > 0,
            "auto-pull should have delivered some materials to the construction site"
        );
    }
}
