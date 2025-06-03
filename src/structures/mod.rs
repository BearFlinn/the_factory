pub mod construction;
pub mod placement;
pub mod systems;
pub mod validation;

pub use construction::*;
pub use placement::*;
pub use systems::*;
pub use validation::*;

use bevy::{prelude::*, ui::update};
use crate::grid::{ExpandGridEvent, NewCellEvent};

pub struct BuildingsPlugin;

pub fn setup(mut commands: Commands) {
    commands.insert_resource(BuildingRegistry::new());
    commands.insert_resource(PowerGrid::default());
    commands.insert_resource(TotalProduction { ore: 800 });
}

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<ExpandGridEvent>()
            .add_event::<NewCellEvent>()
            .add_event::<NetworkChangedEvent>()
            .add_systems(Startup, setup)
            .add_systems(Update, (
                handle_building_input,
                place_building,
                remove_building,
                update_producers,
                update_power_grid,
                validate_placement,
                update_operational_status,
                update_operational_indicators,
                update_network_connections
            ));
    }
}
