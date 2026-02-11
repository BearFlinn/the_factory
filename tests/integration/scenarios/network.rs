use the_factory::{
    structures::NetWorkComponent, systems::NetworkConnectivity,
    workers::pathfinding::calculate_path,
};

use crate::harness::*;

#[test]
fn hub_is_connected_at_origin() {
    let mut app = headless_app();
    tick(&mut app);

    let network = app.world().resource::<NetworkConnectivity>();
    assert!(
        network.is_core_network_cell(0, 0),
        "hub center not in core network"
    );
    assert!(
        network.is_core_network_cell(1, 0),
        "hub cell (1,0) not in core network"
    );
    assert!(
        network.is_core_network_cell(-1, 0),
        "hub cell (-1,0) not in core network"
    );
    assert!(
        network.is_core_network_cell(0, 1),
        "hub cell (0,1) not in core network"
    );
    assert!(
        network.is_core_network_cell(0, -1),
        "hub cell (0,-1) not in core network"
    );
}

#[test]
fn connector_extends_network() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    let connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let network = app.world().resource::<NetworkConnectivity>();
    assert!(
        network.is_core_network_cell(2, 0),
        "connector at (2,0) should be in core network"
    );
    assert_has_component::<NetWorkComponent>(app.world(), connector);
}

#[test]
fn disconnected_building_is_not_operational() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(20, 20)]);

    let smelter = spawn_building(&mut app, "Smelter", 20, 20);
    tick_n(&mut app, 5);

    assert_not_operational(app.world(), smelter);
}

#[test]
fn pathfinding_works_across_connected_network() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world();
    let network = world.resource::<NetworkConnectivity>();
    assert!(network.is_cell_connected(0, 0));
    assert!(network.is_cell_connected(1, 0));

    let grid = world.resource::<the_factory::grid::Grid>();
    let path = calculate_path((0, 0), (1, 0), network, grid);
    assert!(path.is_some(), "should find path between connected cells");
}

#[test]
fn pathfinding_fails_across_gap() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(20, 20)]);

    let world = app.world();
    let network = world.resource::<NetworkConnectivity>();
    let grid = world.resource::<the_factory::grid::Grid>();

    let path = calculate_path((0, 0), (20, 20), network, grid);
    assert!(path.is_none(), "should not find path to disconnected cell");
}

#[test]
fn network_updates_after_building_removal() {
    let mut app = headless_app();
    tick(&mut app);

    let world = app.world_mut();
    ensure_grid_coordinates(world, &[(2, 0)]);

    let connector = spawn_building(&mut app, "Connector", 2, 0);
    tick_n(&mut app, 3);

    let network = app.world().resource::<NetworkConnectivity>();
    assert!(
        network.is_core_network_cell(2, 0),
        "connector should be in network before removal"
    );

    app.world_mut().despawn(connector);
    app.world_mut()
        .write_message(the_factory::systems::NetworkChangedEvent);
    tick_n(&mut app, 3);

    let network = app.world().resource::<NetworkConnectivity>();
    assert!(
        !network.is_core_network_cell(2, 0),
        "removed connector cell should no longer be in core network"
    );
}
