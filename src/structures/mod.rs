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

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);

        match BuildingRegistry::load_from_assets() {
            Ok(registry) => {
                app.insert_resource(registry);
            }
            Err(e) => {
                error!("failed to load building registry: {e}");
            }
        }

        app.add_message::<PlaceBuildingRequestEvent>()
            .add_message::<PlaceBuildingValidationEvent>()
            .add_message::<RemoveBuildingEvent>()
            .init_resource::<construction_auto_pull::ConstructionAutoPullTimer>()
            .add_systems(Startup, place_hub)
            .add_systems(
                Update,
                (
                    handle_building_input
                        .in_set(BuildingSystemSet::Input)
                        .run_if(not(in_state(crate::ui::UiMode::WorkflowCreate))),
                    validate_placement.in_set(BuildingSystemSet::Validation),
                    (
                        place_building,
                        monitor_construction_completion,
                        handle_building_view_range_expansion,
                        assign_drill_recipes.run_if(drill_awaiting_assignment),
                        remove_building,
                    )
                        .chain()
                        .in_set(BuildingSystemSet::Placement),
                    ((
                        commitment::evaluate_recipe_commitments
                            .run_if(commitment::any_needs_evaluation),
                        commitment::commit_pending_recipes,
                        sync_input_port_limits,
                        update_port_crafters,
                        update_source_port_crafters,
                        update_sink_port_crafters,
                        construction_auto_pull::auto_pull_construction_materials,
                    )
                        .chain())
                    .in_set(BuildingSystemSet::Operations),
                ),
            );
    }
}
