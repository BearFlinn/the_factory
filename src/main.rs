use bevy::prelude::*;

mod structures;
mod grid;
mod ui;
mod camera;
mod resources;

pub use grid::Grid;
use grid::{setup_grid, spawn_grid, handle_grid_expansion};
use camera::CameraPlugin;

fn main() {
    App::new()
        .add_event::<grid::NewCellEvent>()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            CameraPlugin,
            structures::BuildingsPlugin, 
            ui::UIPlugin, 
            resources::ResourcesPlugin
        ))
        .add_systems(Startup, (
            setup_grid, 
            spawn_grid, 
            structures::place_hub
        ).chain())
        .add_systems(Update, handle_grid_expansion)
        .run();
}
