use std::collections::HashSet;
use bevy::prelude::*;

#[derive(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32
}

#[derive(Component)]
pub struct Layer(pub i32);

#[derive(Component)]
pub struct CellChildren(pub Vec<Entity>);

#[derive(Bundle)]
pub struct GridCell {
    pub position: Position,
    pub children: CellChildren
}

#[derive(Debug, Clone, Copy)]
pub struct GridCoordinates {
    pub grid_x: i32,
    pub grid_y: i32,
    pub world_x: f32,
    pub world_y: f32,
}

#[derive(Resource)]
pub struct Grid {
    pub cell_size: f32,
    pub valid_coordinates: HashSet<(i32, i32)>,
}

impl Grid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            valid_coordinates: HashSet::new(),
        }
    }

    pub fn add_coordinate(&mut self, x: i32, y: i32) -> bool {
        self.valid_coordinates.insert((x, y))
    }
    
    pub fn get_adjacent_positions(x: i32, y: i32) -> [(i32, i32); 4] {
        [(x+1, y), (x-1, y), (x, y+1), (x, y-1)]
    }

    pub fn get_cursor_grid_coordinates(
        &self,
        windows: &Query<&Window>,
        camera_q: &Query<(&Camera, &GlobalTransform)>,
    ) -> Option<GridCoordinates> {
        let window = windows.single();
        let (camera, camera_transform) = camera_q.single();
        
        if let Some(world_position) = window.cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
            .map(|ray| ray.origin.truncate())
        {
            self.world_to_grid_coordinates(world_position)
        } else {
            None
        }
    }

    pub fn world_to_grid_coordinates(&self, world_position: Vec2) -> Option<GridCoordinates> {
        let grid_x = (world_position.x / self.cell_size).round() as i32;
        let grid_y = (world_position.y / self.cell_size).round() as i32;
        
        if self.valid_coordinates.contains(&(grid_x, grid_y)) {
            let world_x = grid_x as f32 * self.cell_size;
            let world_y = grid_y as f32 * self.cell_size;
            
            Some(GridCoordinates {
                grid_x,
                grid_y,
                world_x,
                world_y,
            })
        } else {
            None
        }
    }

    pub fn grid_to_world_coordinates(&self, grid_x: i32, grid_y: i32) -> Vec2 {
        let world_x = grid_x as f32 * self.cell_size;
        let world_y = grid_y as f32 * self.cell_size;
        
        Vec2::new(world_x, world_y)
    }

    pub fn get_coordinates_in_radius(&self, center_x: i32, center_y: i32, radius: i32) -> Vec<(i32, i32)> {
        let mut coordinates = Vec::new();
        
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                if distance <= radius as f32 {
                    coordinates.push((center_x + dx, center_y + dy));
                }
            }
        }
        
        coordinates
    }
}

pub fn setup_grid(mut commands: Commands) {
    commands.insert_resource(Grid::new(64.0));
}

pub fn spawn_grid(
    mut commands: Commands,
    mut grid: ResMut<Grid>,
) {
    for y in -2..=2 {
        for x in -2..=2 {
            grid.valid_coordinates.insert((x, y));
            spawn_cell(&mut commands, &grid, x, y);
        }
    }
}

#[derive(Event)]
pub struct NewCellEvent {
    pub x: i32,
    pub y: i32,
    pub entity: Entity
}

fn spawn_cell(
    commands: &mut Commands, 
    grid: &Grid, 
    x: i32, 
    y: i32,
) -> Entity {
    let grid_line_width = 2.0;
    let cell_visual_size = grid.cell_size - grid_line_width;
    
    let pos_x = x as f32 * grid.cell_size;
    let pos_y = y as f32 * grid.cell_size;

    let cell_entity = commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::new(grid.cell_size, grid.cell_size)),
        Transform::from_xyz(pos_x, pos_y, 0.0),
        GridCell { 
            position: Position { x, y }, 
            children: CellChildren(Vec::new())
        }
    )).id();
    
    commands.entity(cell_entity).with_children(|parent| {
        parent.spawn((
            Sprite::from_color(Color::WHITE, Vec2::new(cell_visual_size, cell_visual_size)),
            Transform::from_xyz(0.0, 0.0, 0.1),
        ));
    });
    cell_entity
}

#[derive(Event)]
pub struct ExpandGridEvent {
    pub center_x: i32,
    pub center_y: i32,
    pub radius: i32,
}

pub fn handle_grid_expansion(
    mut commands: Commands,
    mut expand_events: EventReader<ExpandGridEvent>,
    mut grid: ResMut<Grid>,
    mut cell_event: EventWriter<NewCellEvent>,
) {
    for event in expand_events.read() {
        let new_coordinates = grid.get_coordinates_in_radius(event.center_x, event.center_y, event.radius);
        for (x, y) in new_coordinates {
            if !grid.valid_coordinates.contains(&(x, y)) {
                grid.add_coordinate(x, y);
                let new_cell =spawn_cell(&mut commands, &grid, x, y);
                cell_event.send(NewCellEvent { x, y, entity: new_cell });
            }
        }
    }
}
