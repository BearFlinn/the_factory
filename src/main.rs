use bevy::prelude::*;

mod buildings;
mod grid;
mod ui;

pub use grid::Grid;
use grid::{spawn_grid};

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((buildings::BuildingsPlugin, ui::UIPlugin))
        .add_systems(Startup, (setup_camera, spawn_grid))
        .run();
}
