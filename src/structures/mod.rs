pub mod construction;
pub mod placement;
pub mod systems;

pub use construction::*;
pub use placement::*;
pub use systems::*;

use bevy::prelude::*;
use crate::grid::ExpandGridEvent;

pub struct BuildingsPlugin;

pub fn setup(mut commands: Commands) {
    commands.insert_resource(BuildingRegistry::new());
    commands.insert_resource(PowerGrid::default());
    commands.insert_resource(TotalProduction::default());
}

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<PlaceBuildingEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<ExpandGridEvent>()
            .add_systems(Startup, setup)
            .add_systems(Update, (
                handle_building_input,
                place_building,
                remove_building,
                update_producers,
                update_power_grid
            ));
    }
}
