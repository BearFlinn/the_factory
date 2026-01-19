pub mod building_config;
pub mod construction;
pub mod placement;
pub mod production;
pub mod validation;

pub use construction::*;
pub use placement::*;
pub use production::*;
pub use validation::*;

use bevy::prelude::*;

// ============================================================================
// Building Archetype Markers
// ============================================================================
// These marker components identify a building's role in the production chain.
// They replace the complex InventoryTypes enum with simple, composable markers.

/// Marker for buildings that only produce items (e.g., Mining Drill).
/// Source buildings have only an output buffer - they don't consume items.
#[derive(Component, Debug, Default)]
pub struct Source;

/// Marker for buildings that transform items (e.g., Smelter, Assembler).
/// Processor buildings have both input and output buffers.
#[derive(Component, Debug, Default)]
pub struct Processor;

/// Marker for buildings that consume items for non-item output (e.g., Generator).
/// Sink buildings have only an input buffer - they consume items but don't produce items.
#[derive(Component, Debug, Default)]
pub struct Sink;

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
    commands.insert_resource(BuildingRegistry::load_from_assets());
}

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        configure_building_system_sets(app);

        app.add_event::<PlaceBuildingRequestEvent>()
            .add_event::<PlaceBuildingValidationEvent>()
            .add_event::<RemoveBuildingEvent>()
            .add_event::<CrafterLogisticsRequest>()
            .add_event::<ConstructionMaterialRequest>()
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
                        update_recipe_crafters,
                        crafter_logistics_requests,
                        handle_recipe_selection_logistics,
                    )
                        .chain())
                    .in_set(BuildingSystemSet::Operations),
                ),
            );
    }
}
