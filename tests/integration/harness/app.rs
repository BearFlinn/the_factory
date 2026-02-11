use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

use the_factory::{
    configure_system_sets,
    grid::GridPlugin,
    invariants::InvariantPlugin,
    materials::MaterialsPlugin,
    structures::BuildingsPlugin,
    systems::SystemsPlugin,
    ui::{SelectedBuilding, UiMode},
    workers::WorkersPlugin,
};

pub fn headless_app() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));

    configure_system_sets(&mut app);

    app.init_state::<UiMode>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<SelectedBuilding>();

    app.add_plugins((
        GridPlugin,
        MaterialsPlugin,
        SystemsPlugin,
        BuildingsPlugin,
        WorkersPlugin,
    ));

    app.add_plugins(InvariantPlugin);

    app
}
