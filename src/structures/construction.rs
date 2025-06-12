
use crate::materials::{RecipeDef, RecipeName};
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


#[derive(Component)]
#[allow(dead_code)]
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
        Operational(true),
        Layer(BUILDING_LAYER),
    ))
    .insert(Sprite::from_color(Color::srgb(0.3, 0.3, 0.7), Vec2::new(120.0, 120.0)))
    .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
    .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}
