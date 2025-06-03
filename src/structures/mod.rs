pub mod construction;
pub mod placement;
pub mod systems;
pub mod validation;

pub use construction::*;
pub use placement::*;
pub use systems::*;
pub use validation::*;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum BuildingSystemSet {
    Input,
    Validation,
    Placement,
    Operations,
}

fn configure_building_system_sets(app: &mut App) {
    app.configure_sets(Update, (
        BuildingSystemSet::Input,
        BuildingSystemSet::Validation,
        BuildingSystemSet::Placement,
        BuildingSystemSet::Operations,
    ).chain().in_set(crate::GameplaySet::BuildingOperations));
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(BuildingRegistry::new());
    commands.insert_resource(PowerGrid::default());
    commands.insert_resource(TotalProduction { ore: 800 });
    commands.insert_resource(NetworkConnectivity::default());
}

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);
        
        app
            .add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<NetworkChangedEvent>()
            .add_systems(Startup, (
                setup,
                place_hub,
            ).chain())
            .add_systems(Update, (
                handle_building_input
                    .in_set(BuildingSystemSet::Input),
                
                validate_placement
                    .in_set(BuildingSystemSet::Validation),
                
                (place_building, remove_building)
                    .in_set(BuildingSystemSet::Placement),
                
                (
                    update_producers,
                    update_power_grid,
                    update_network_connectivity,
                    update_operational_status_optimized,
                    update_operational_indicators,
                    update_visual_network_connections,
                ).in_set(BuildingSystemSet::Operations),
            ));
    }
}
