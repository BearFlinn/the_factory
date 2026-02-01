pub use crate::{
    constants::gridlayers::BUILDING_LAYER,
    grid::{CellChildren, Grid, Layer, Position},
    materials::items::{InputPort, InventoryAccess, StoragePort},
    structures::building_config::*,
    systems::Operational,
};
use crate::{
    constants::structures::MINING_DRILL,
    grid::ExpandGridEvent,
    materials::{RecipeDef, RecipeName},
    resources::{ResourceNode, ResourceNodeRecipe},
    systems::NetworkChangedEvent,
};

#[derive(Component)]
pub struct Building;

#[derive(Component)]
pub struct ViewRange {
    pub radius: i32,
}

#[derive(Component, Debug)]
pub struct RecipeCrafter {
    pub timer: Timer,
    pub current_recipe: Option<RecipeName>,
    pub available_recipes: Vec<RecipeName>,
}

#[derive(Component, Debug, Default, Clone)]
pub struct RecipeCommitment {
    pub committed_recipe: Option<RecipeName>,
    pub pending_recipe: Option<RecipeName>,
}

#[derive(Component)]
pub struct NeedsRecipeCommitmentEvaluation;

impl RecipeCommitment {
    pub fn new_committed(recipe: Option<RecipeName>) -> Self {
        Self {
            committed_recipe: recipe,
            pending_recipe: None,
        }
    }
}

impl RecipeCrafter {
    pub fn is_single_recipe(&self) -> bool {
        self.available_recipes.is_empty()
    }

    pub fn is_multi_recipe(&self) -> bool {
        !self.available_recipes.is_empty()
    }

    pub fn get_active_recipe(&self) -> Option<&RecipeName> {
        self.current_recipe.as_ref()
    }

    pub fn set_recipe(&mut self, recipe_name: RecipeName) -> Result<(), String> {
        if self.is_single_recipe() || self.available_recipes.contains(&recipe_name) {
            self.current_recipe = Some(recipe_name);
            Ok(())
        } else {
            Err(format!(
                "Recipe '{recipe_name}' not available for this crafter"
            ))
        }
    }
}

#[derive(Component)]
pub struct PowerGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct PowerConsumer {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeConsumer {
    pub amount: i32,
}

#[derive(Component, Clone)]
pub struct BuildingCost {
    pub cost: RecipeDef,
}

#[derive(Component)]
pub struct MultiCellBuilding {
    pub width: i32,
    pub height: i32,
    pub center_x: i32,
    pub center_y: i32,
}

#[derive(Component, PartialEq)]
pub struct NetWorkComponent;

#[derive(Component)]
pub struct PendingDrillRecipeAssignment {
    pub position: Position,
}

#[derive(Component)]
pub struct ConstructionSite {
    pub building_name: String,
}

#[derive(Bundle)]
pub struct ConstructionSiteBundle {
    pub construction_site: ConstructionSite,
    pub building_cost: BuildingCost,
    input_port: InputPort,
    pub position: Position,
    pub layer: Layer,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl ConstructionSiteBundle {
    pub fn new(
        building_name: String,
        building_cost: BuildingCost,
        position: Position,
        world_pos: Vec2,
        appearance: &AppearanceDef,
    ) -> Self {
        Self {
            construction_site: ConstructionSite { building_name },
            building_cost,
            input_port: InputPort::new(1000),
            position,
            layer: Layer(BUILDING_LAYER),
            sprite: Sprite::from_color(
                Color::srgba(
                    appearance.color.0,
                    appearance.color.1,
                    appearance.color.2,
                    0.7,
                ),
                Vec2::new(appearance.size.0, appearance.size.1),
            ),
            transform: Transform::from_xyz(world_pos.x, world_pos.y, 0.8),
        }
    }
}

impl BuildingRegistry {
    pub fn load_from_assets() -> Result<Self, Box<dyn std::error::Error>> {
        let ron_content = include_str!("../assets/buildings.ron");
        Self::from_ron(ron_content)
    }
}

// TODO: Improve Multi-cell building implementation

pub fn occupy_area(
    grid_cells: &mut Query<(Entity, &Position, &mut CellChildren)>,
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
    building_entity: Entity,
) {
    let half_width = width / 2;
    let half_height = height / 2;

    for dy in -half_height..=half_height {
        for dx in -half_width..=half_width {
            let check_x = center_x + dx;
            let check_y = center_y + dy;

            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == check_x && pos.y == check_y)
            {
                cell_children.0.push(building_entity);
            }
        }
    }
}

#[derive(Component)]
pub struct Hub;

#[derive(Component)]
pub struct Launchpad;

pub fn place_hub(
    mut commands: Commands,
    grid: Res<Grid>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    let center_x = 0;
    let center_y = 0;

    let world_pos = grid.grid_to_world_coordinates(center_x, center_y);

    let mut storage_port = StoragePort::new(10000);
    storage_port.add_item("Iron Ore", 400);
    storage_port.add_item("Copper Ore", 400);

    let building_entity = commands
        .spawn((
            Building,
            Hub,
            Position {
                x: center_x,
                y: center_y,
            },
            MultiCellBuilding {
                width: 3,
                height: 3,
                center_x,
                center_y,
            },
            PowerGenerator { amount: 100 },
            ComputeGenerator { amount: 60 },
            storage_port,
            Operational(None),
            Layer(BUILDING_LAYER),
        ))
        .insert(Sprite::from_color(
            Color::srgb(0.3, 0.3, 0.7),
            Vec2::new(120.0, 120.0),
        ))
        .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
        .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}

pub fn monitor_construction_completion(
    mut commands: Commands,
    construction_sites: Query<
        (
            Entity,
            &ConstructionSite,
            &InputPort,
            &BuildingCost,
            &Position,
            &Transform,
        ),
        (Changed<InputPort>, With<ConstructionSite>),
    >,
    registry: Res<BuildingRegistry>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    mut network_events: MessageWriter<NetworkChangedEvent>,
) {
    for (site_entity, construction_site, input_port, building_cost, position, transform) in
        &construction_sites
    {
        if input_port.has_items_for_recipe(&building_cost.cost.inputs) {
            commands.entity(site_entity).despawn();

            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y)
            {
                cell_children.0.retain(|&entity| entity != site_entity);
            }

            if let Some(building_entity) = registry.spawn_building(
                &mut commands,
                &construction_site.building_name,
                position.x,
                position.y,
                transform.translation.truncate(),
            ) {
                if construction_site.building_name == MINING_DRILL {
                    commands
                        .entity(building_entity)
                        .insert(PendingDrillRecipeAssignment {
                            position: *position,
                        });
                }
                if let Some((_, _, mut cell_children)) = grid_cells
                    .iter_mut()
                    .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y)
                {
                    cell_children.0.push(building_entity);
                }

                network_events.write(NetworkChangedEvent);
                println!(
                    "Construction completed: {} at ({}, {})",
                    construction_site.building_name, position.x, position.y
                );
            }
        }
    }
}

pub fn assign_drill_recipes(
    mut commands: Commands,
    mut drills: Query<(Entity, &mut RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
    resource_nodes: Query<(&ResourceNodeRecipe, &Position), With<ResourceNode>>,
) {
    for (drill_entity, mut recipe_crafter, pending) in &mut drills {
        if let Some((resource_recipe, _)) = resource_nodes
            .iter()
            .find(|(_, pos)| pos.x == pending.position.x && pos.y == pending.position.y)
        {
            if let Err(error) = recipe_crafter.set_recipe(resource_recipe.recipe_name.clone()) {
                println!(
                    "Failed to assign recipe to drill at ({}, {}): {}",
                    pending.position.x, pending.position.y, error
                );
            } else {
                commands
                    .entity(drill_entity)
                    .remove::<PendingDrillRecipeAssignment>();
                println!(
                    "Assigned recipe '{}' to drill at ({}, {})",
                    resource_recipe.recipe_name, pending.position.x, pending.position.y
                );
            }
        }
    }
}

pub fn drill_awaiting_assignment(
    drills: Query<(&RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
) -> bool {
    !drills.is_empty()
}

pub fn handle_building_view_range_expansion(
    buildings_with_view_range: Query<(&ViewRange, &Position), Added<Building>>,
    mut expand_events: MessageWriter<ExpandGridEvent>,
) {
    for (view_range, position) in &buildings_with_view_range {
        if view_range.radius > 0 {
            expand_events.write(ExpandGridEvent {
                center_x: position.x,
                center_y: position.y,
                radius: view_range.radius,
            });

            println!(
                "Expanding grid for building at ({}, {}) with radius {}",
                position.x, position.y, view_range.radius
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn recipe_commitment_new_committed_with_recipe() {
        let commitment = RecipeCommitment::new_committed(Some("iron_ingot".to_string()));

        assert_eq!(commitment.committed_recipe, Some("iron_ingot".to_string()));
        assert_eq!(commitment.pending_recipe, None);
    }

    #[test]
    fn recipe_commitment_new_committed_without_recipe() {
        let commitment = RecipeCommitment::new_committed(None);

        assert_eq!(commitment.committed_recipe, None);
        assert_eq!(commitment.pending_recipe, None);
    }
}
