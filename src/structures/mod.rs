pub mod building_config;
pub mod commitment;
pub mod construction;
pub mod construction_auto_pull;
pub mod placement;
pub mod production;
pub mod validation;

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
    app.configure_sets(
        Update,
        (
            BuildingSystemSet::Input,
            BuildingSystemSet::Validation,
            BuildingSystemSet::Placement,
            BuildingSystemSet::Operations,
        )
            .chain()
            .in_set(crate::GameplaySet::DomainOperations),
    );
}

pub fn setup(mut commands: Commands) {
    if let Ok(registry) = BuildingRegistry::load_from_assets() {
        commands.insert_resource(registry);
    }
}

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);

        app.add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<PortLogisticsRequest>()
            .add_event::<ConstructionMaterialRequest>()
            .init_resource::<PortLogisticsTimer>()
            .add_systems(Startup, (setup, place_hub).chain())
            .add_systems(
                Update,
                (
                    handle_building_input.in_set(BuildingSystemSet::Input),
                    validate_placement.in_set(BuildingSystemSet::Validation),
                    (
                        place_building,
                        monitor_construction_progress,
                        monitor_construction_completion,
                        handle_building_view_range_expansion,
                        assign_drill_recipes.run_if(drill_awaiting_assignment),
                        remove_building,
                    )
                        .chain()
                        .in_set(BuildingSystemSet::Placement),
                    ((
                        // Recipe commitment systems
                        commitment::evaluate_recipe_commitments
                            .run_if(commitment::any_needs_evaluation),
                        commitment::commit_pending_recipes,
                        // Port-based crafting systems
                        update_port_crafters,
                        update_source_port_crafters,
                        update_sink_port_crafters,
                        poll_port_logistics,
                    )
                        .chain())
                    .in_set(BuildingSystemSet::Operations),
                ),
            );
    }
}
