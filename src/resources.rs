use bevy::prelude::*;
use rand::{thread_rng, Rng};
use crate::{
    constants::{gridlayers::RESOURCE_LAYER, items::*}, grid::{CellChildren, Grid, Layer, NewCellEvent, Position}, materials::RecipeName
};

#[derive(Component)]
pub struct ResourceNode;

#[derive(Component)]
pub struct ResourceNodeRecipe {
    pub recipe_name: RecipeName
}

#[derive(Bundle)]
pub struct ResourceNodeBundle {
    node: ResourceNode,
    produces: ResourceNodeRecipe,
    position: Position,
    layer: Layer,
}

impl ResourceNodeBundle {
    pub fn new(x: i32, y: i32, produces: ResourceNodeRecipe) -> Self {
        ResourceNodeBundle {
            node: ResourceNode,
            produces,
            position: Position { x, y },
            layer: Layer(RESOURCE_LAYER),
        }
    }
}

const IRON_ORE_PROBABILITY: f32 = 0.4;   
const COPPER_ORE_PROBABILITY: f32 = 0.3;   
// const COAL_PROBABILITY: f32 = 0.3;       

fn select_random_ore() -> (RecipeName, Color) {
    let mut rng = thread_rng();
    let roll = rng.gen::<f32>();
    
    if roll < IRON_ORE_PROBABILITY {
        (IRON_ORE.to_string(), Color::srgb(0.2, 0.3, 0.5))
    } else if roll < IRON_ORE_PROBABILITY + COPPER_ORE_PROBABILITY {
        (COPPER_ORE.to_string(), Color::srgb(0.8, 0.5, 0.2)) 
    } else {
        (COAL.to_string(), Color::srgb(0.2, 0.2, 0.2)) 
    }
}

// TODO: Add clustering
pub fn spawn_resource_node(
    mut commands: Commands,
    grid: Res<Grid>,
    mut cell_event: EventReader<NewCellEvent>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    for event in cell_event.read() {
        let spawn_resource = thread_rng().gen::<f32>() < 0.028;
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

        let (recipe_name, color) = select_random_ore();

        let resource_node = commands.spawn(
            ResourceNodeBundle::new(
                event.x,
                event.y,
                ResourceNodeRecipe { recipe_name },
            ))
            .insert(Sprite::from_color(color, Vec2::new(48.0, 48.0)))
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
