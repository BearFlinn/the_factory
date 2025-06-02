use bevy::prelude::*;
use rand::{thread_rng, Rng};
use crate::grid::{CellChildren, Grid, Layer, NewCellEvent, Position};

const RESOURCE_LAYER: i32 = 0;

#[derive(Component)]
pub struct RawMaterial;

#[derive(Component)]
pub enum MaterialType {
    Ore,
}

#[derive(Bundle)]
pub struct ResourceNode {
    resource: RawMaterial,
    resource_type: MaterialType,
    position: Position,
    layer: Layer,
}

impl ResourceNode {
    pub fn new(x: i32, y: i32, resource_type: MaterialType) -> Self {
        ResourceNode {
            resource: RawMaterial,
            resource_type,
            position: Position { x, y },
            layer: Layer(RESOURCE_LAYER),
        }
    }
}

pub fn spawn_resource_node(
    mut commands: Commands,
    grid: Res<Grid>,
    mut cell_event: EventReader<NewCellEvent>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    for event in cell_event.read() {
        let spawn_resource = thread_rng().gen::<f32>() < 0.02;
        let world_pos = grid.grid_to_world_coordinates(event.x, event.y);
        
        if !spawn_resource {
            continue;
        }

        let resource_node = commands.spawn(
            ResourceNode::new(event.x, event.y, MaterialType::Ore)
            )
            .insert(Sprite::from_color(Color::srgb(0.7, 0.3, 0.3), Vec2::new(48.0, 48.0)))
            .insert(Transform::from_xyz(world_pos.x, world_pos.y, 0.2)).id();
        
        let Some((_, _, mut cell_children)) = grid_cells
            .iter_mut()
            .find(|(_, pos, _)| pos.x == event.x && pos.y == event.y) else {
            continue;
        };

        cell_children.0.push(resource_node);
        eprintln!("spawned resource node at ({}, {})", event.x, event.y);
    }
}

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NewCellEvent>()
            .add_systems(Update, spawn_resource_node);
    }
}