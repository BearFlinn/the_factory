use bevy::prelude::*;
use std::collections::HashMap;
pub use crate::{
    grid::{CellChildren, Grid, Layer, Position}, 
    structures::{BUILDING_LAYER, building_config::*},
    items::{Inventory, create_ore_item},
    systems::Operational
};

#[derive(Component)]
pub struct Building {
    pub id: BuildingId,
}

#[derive(Component)]
pub struct Name {
    pub name: String,
}

#[derive(Component)]
pub struct ViewRange {
    pub radius: i32,
}

#[derive(Component)]
pub struct Producer {
    pub amount: u32,
    pub timer: Timer,
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
pub struct ResourceConsumer {
    pub amount: u32,
    pub timer: Timer,
}

// TODO: Update to use receipes
#[derive(Component)]
pub struct BuildingCost {
    pub ore: u32,
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

    // Create central inventory with starting ore
    let mut central_inventory = Inventory::new(10000); // Large capacity for central storage
    central_inventory.add_item(create_ore_item(), 800); // Starting ore amount

    let building_entity = commands.spawn((
        Building { id: HUB },
        Hub,
        Name { name: "Command Hub".to_string() },
        Position { x: center_x, y: center_y },
        MultiCellBuilding { 
            width: 3, 
            height: 3, 
            center_x, 
            center_y 
        },
        PowerGenerator { amount: 100 },
        ComputeGenerator { amount: 60 },
        central_inventory, // Add the central inventory
        Operational(true),
        Layer(BUILDING_LAYER),
    ))
    .insert(Sprite::from_color(Color::srgb(0.3, 0.3, 0.7), Vec2::new(120.0, 120.0)))
    .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
    .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}
