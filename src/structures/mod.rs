pub mod construction;
pub mod placement;
pub mod production;
pub mod validation;
pub mod building_config;

pub use construction::*;
pub use placement::*;
pub use production::*;
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
    ).chain().in_set(crate::GameplaySet::DomainOperations));
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(BuildingRegistry::load_from_assets());
}

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);
        
        app
            .add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<CrafterLogisticsRequest>()
            .add_event::<ConstructionMaterialRequest>()
            .add_systems(Startup, (
                setup,
                place_hub,
            ).chain())
            .add_systems(Update, (
                handle_building_input
                    .in_set(BuildingSystemSet::Input),
                
                validate_placement
                    .in_set(BuildingSystemSet::Validation),
                
                (
                place_building,
                monitor_construction_progress,
                monitor_construction_completion,
                handle_building_view_range_expansion,
                assign_drill_recipes.run_if(drill_awaiting_assignment),
                remove_building
                ).chain()
                    .in_set(BuildingSystemSet::Placement),
                ((
                update_recipe_crafters, 
                crafter_logistics_requests,
                handle_recipe_selection_logistics,
                ).chain()
                )
                    .in_set(BuildingSystemSet::Operations),
            ));
    }
}
