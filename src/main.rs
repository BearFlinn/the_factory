use bevy::prelude::*;

mod camera;
mod constants;
mod grid;
mod materials;
mod resources;
mod structures;
mod systems;
mod ui;
mod workers;

use camera::CameraPlugin;
use grid::GridPlugin;
use materials::MaterialsPlugin;
use resources::ResourcesPlugin;
use structures::BuildingsPlugin;
use systems::SystemsPlugin;
use ui::UIPlugin;
use workers::WorkersPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameplaySet {
    GridUpdate,
    ResourceSpawning,
    SystemsUpdate,
    DomainOperations,
    UIUpdate,
}

pub fn configure_system_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            GameplaySet::GridUpdate,
            GameplaySet::ResourceSpawning,
            GameplaySet::SystemsUpdate,
            GameplaySet::DomainOperations,
            GameplaySet::UIUpdate,
        )
            .chain(),
    );
}

fn main() {
    let mut app = App::new();
    configure_system_sets(&mut app);
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            GridPlugin,
            ResourcesPlugin,
            MaterialsPlugin,
            SystemsPlugin,
            BuildingsPlugin,
            WorkersPlugin,
            CameraPlugin,
            UIPlugin,
        ))
        .run();
}
