use bevy::prelude::*;
use rand::{thread_rng, Rng};
use crate::{
    grid::{CellChildren, Grid, Layer, NewCellEvent, Position},
    items::Item,
};

const RESOURCE_LAYER: i32 = 0;

#[derive(Component)]
pub struct ResourceNode;

#[derive(Bundle)]
pub struct ResourceNodeBundle {
    node: ResourceNode,
    produces: Item,
    position: Position,
    layer: Layer,
}

impl ResourceNodeBundle {
    pub fn new(x: i32, y: i32, produces: Item) -> Self {
        ResourceNodeBundle {
            node: ResourceNode,
            produces,
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
        let spawn_resource = thread_rng().gen::<f32>() < 0.025;
        let world_pos = grid.grid_to_world_coordinates(event.x, event.y);
        
        if !spawn_resource {
            continue;
        }

        let Some((_, _, mut cell_children)) = grid_cells
            .iter_mut()
            .find(|(_, pos, _)| pos.x == event.x && pos.y == event.y) else {
            eprintln!("could not find cell at ({}, {})", event.x, event.y);
            continue;
        };

        let resource_node = commands.spawn(
            ResourceNodeBundle::new(
                event.x,
                event.y,
                Item {
                    id: 0,
                    name: "Ore".to_string(),
                },
            ))
            .insert(Sprite::from_color(Color::srgb(0.7, 0.3, 0.3), Vec2::new(48.0, 48.0)))
            .insert(Transform::from_xyz(world_pos.x, world_pos.y, 0.2)).id();


        cell_children.0.push(resource_node);
        eprintln!("spawned resource node at ({}, {})", event.x, event.y);
    }
}

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_resource_node);
    }
}