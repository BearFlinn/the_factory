
use std::collections::HashMap;

use crate::{constants::structures::MINING_DRILL, grid::ExpandGridEvent, materials::{ItemName, RecipeDef, RecipeName}, resources::{ResourceNode, ResourceNodeRecipe}, systems::NetworkChangedEvent, workers::Priority};
pub use crate::{
    grid::{CellChildren, Grid, Layer, Position}, 
    structures::{building_config::*},
    materials::items::{Inventory, InventoryType, InventoryTypes},
    systems::Operational,
    constants::gridlayers::BUILDING_LAYER,
};

#[derive(Component)]
pub struct Building;

#[derive(Component)]
#[allow(dead_code)] // TODO: Figure out if this is needed
pub struct ViewRange {
    pub radius: i32,
}

#[derive(Component)]
pub struct RecipeCrafter {
    pub timer: Timer,
    pub recipe: RecipeName,
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
    pub inventory: Inventory,
    pub inventory_type: InventoryType,
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
            inventory: Inventory::new(1000), // Large capacity for construction materials
            inventory_type: InventoryType(InventoryTypes::Requester),
            position,
            layer: Layer(BUILDING_LAYER),
            sprite: Sprite::from_color(
                Color::srgba(
                    appearance.color.0, // Dimmed colors for construction
                    appearance.color.1,
                    appearance.color.2,
                    0.7, // Semi-transparent
                ),
                Vec2::new(appearance.size.0, appearance.size.1),
            ),
            transform: Transform::from_xyz(world_pos.x, world_pos.y, 0.8), // Slightly lower Z than buildings
        }
    }
}

#[derive(Event)]
pub struct ConstructionMaterialRequest {
    pub site: Entity,
    pub position: Position,
    pub needed_materials: HashMap<ItemName, u32>,
    pub priority: Priority,
}

impl BuildingRegistry {
    pub fn load_from_assets() -> Self {
        let ron_content = include_str!("../assets/buildings.ron");
        Self::from_ron(ron_content).expect("Failed to load building definitions")
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


pub fn place_hub(
    mut commands: Commands,
    grid: Res<Grid>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    let center_x = 0;
    let center_y = 0;

    let world_pos = grid.grid_to_world_coordinates(center_x, center_y);

    let mut central_inventory = Inventory::new(10000);
    central_inventory.add_item("Iron Ore", 400);
    central_inventory.add_item("Copper Ore", 400);

    let building_entity = commands.spawn((
        Building,
        Hub,
        Position { x: center_x, y: center_y },
        MultiCellBuilding { 
            width: 3, 
            height: 3, 
            center_x, 
            center_y 
        },
        PowerGenerator { amount: 100 },
        ComputeGenerator { amount: 60 },
        central_inventory,
        InventoryType (InventoryTypes::Storage),
        Operational(None),
        Layer(BUILDING_LAYER),
    ))
    .insert(Sprite::from_color(Color::srgb(0.3, 0.3, 0.7), Vec2::new(120.0, 120.0)))
    .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
    .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}

pub fn monitor_construction_completion(
    mut commands: Commands,
    construction_sites: Query<(Entity, &ConstructionSite, &Inventory, &BuildingCost, &Position, &Transform), (Changed<Inventory>, With<ConstructionSite>)>,
    registry: Res<BuildingRegistry>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    mut network_events: EventWriter<NetworkChangedEvent>,
    mut expand_events: EventWriter<ExpandGridEvent>,
) {
    for (site_entity, construction_site, inventory, building_cost, position, transform) in construction_sites.iter() {
        // Check if construction site has all required materials
        if inventory.has_items_for_recipe(&building_cost.cost.inputs) {
            // Remove construction site
            commands.entity(site_entity).despawn();
            
            // Update grid cell children to remove construction site
            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y) 
            {
                cell_children.0.retain(|&entity| entity != site_entity);
            }

            // Spawn actual building
            if let Some((building_entity, view_radius)) = registry.spawn_building(
                &mut commands,
                &construction_site.building_name,
                position.x,
                position.y,
                transform.translation.truncate(),
            ) {
                if view_radius > 0 {
                    expand_events.send(ExpandGridEvent {
                        center_x: position.x,
                        center_y: position.y,
                        radius: view_radius,
                    });
                }

                // Set drill recipe
                if &construction_site.building_name == MINING_DRILL {
                    commands.entity(building_entity).insert(PendingDrillRecipeAssignment {
                        position: *position,
                    });
                }
                // Add building to grid cell
                if let Some((_, _, mut cell_children)) = grid_cells
                    .iter_mut()
                    .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y) 
                {
                    cell_children.0.push(building_entity);
                }

                network_events.send(NetworkChangedEvent);
                println!("Construction completed: {} at ({}, {})", 
                         construction_site.building_name, position.x, position.y);
            }
        }
    }
}

pub fn assign_drill_recipes(
    mut commands: Commands,
    mut drills: Query<(Entity, &mut RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
    resource_nodes: Query<(&ResourceNodeRecipe, &Position), With<ResourceNode>>,
) {
    for (drill_entity, mut recipe_crafter, pending) in drills.iter_mut() {
        // Find the resource node at this position
        if let Some((resource_recipe, _)) = resource_nodes
            .iter()
            .find(|(_, pos)| pos.x == pending.position.x && pos.y == pending.position.y)
        {
            recipe_crafter.recipe = resource_recipe.recipe_name.clone();
            commands.entity(drill_entity).remove::<PendingDrillRecipeAssignment>();
            println!("Assigned recipe '{}' to drill at ({}, {})", 
                     resource_recipe.recipe_name, pending.position.x, pending.position.y);
        }
    }
}

pub fn drill_awaiting_assignment(
    drills: Query<(&RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
) -> bool {
    !drills.is_empty()
}