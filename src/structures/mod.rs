pub mod construction;
pub mod placement;
pub mod production;
pub mod validation;
pub mod building_config;

pub use construction::*;
pub use placement::*;
pub use production::*;
pub use validation::*;
pub use building_config::*;

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
    ).chain().in_set(crate::GameplaySet::DomainOperations));
}

pub fn setup(mut commands: Commands) {

}

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);
        
        app
            .add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
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
                    update_resource_consumers,
                ).in_set(BuildingSystemSet::Operations),
            ));
    }
}
