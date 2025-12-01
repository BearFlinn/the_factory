use bevy::prelude::*;

mod structures;
mod grid;
mod ui;
mod camera;
mod resources;
mod workers;
mod materials;
mod systems;
mod constants;

use grid::GridPlugin;
use camera::CameraPlugin;
use structures::BuildingsPlugin;
use ui::UIPlugin;
use resources::ResourcesPlugin;
use workers::WorkersPlugin;
use materials::MaterialsPlugin;
use systems::SystemsPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameplaySet {
    GridUpdate,
    ResourceSpawning,
    SystemsUpdate,
    DomainOperations,
    UIUpdate,
}

pub fn configure_system_sets(app: &mut App) {
    app.configure_sets(Update, (
        GameplaySet::GridUpdate,
        GameplaySet::ResourceSpawning,
        GameplaySet::SystemsUpdate,
        GameplaySet::DomainOperations,
        GameplaySet::UIUpdate,
    ).chain());
}

fn main() {
    let mut app = App::new();
    configure_system_sets(&mut app);
    app
        .add_plugins(DefaultPlugins)
        .add_plugins((
            GridPlugin,
            ResourcesPlugin,
            MaterialsPlugin,
            SystemsPlugin,
            BuildingsPlugin,
            WorkersPlugin,
            CameraPlugin,
            UIPlugin))
        .run();
}
