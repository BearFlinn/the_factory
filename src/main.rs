use bevy::prelude::*;

mod structures;
mod grid;
mod ui;
mod camera;
mod resources;
mod workers;

use grid::GridPlugin;
use camera::CameraPlugin;
use structures::BuildingsPlugin;
use ui::UIPlugin;
use resources::ResourcesPlugin;
use workers::WorkersPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameplaySet {
    /// Core grid operations - must run first
    GridUpdate,
    
    /// Resource spawning - depends on grid events
    ResourceSpawning,
    
    /// All building operations - internally ordered within structures module
    BuildingOperations,
    
    /// UI updates - should run after core gameplay
    UIUpdate,
}

pub fn configure_system_sets(app: &mut App) {
    app.configure_sets(Update, (
        GameplaySet::GridUpdate,
        GameplaySet::ResourceSpawning,
        GameplaySet::BuildingOperations,  // Now encompasses all building sub-phases
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
            BuildingsPlugin,
            WorkersPlugin,
            CameraPlugin,
            UIPlugin))
        .run();
}
