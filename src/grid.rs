use bevy::prelude::*;

#[derive(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32
}

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
    pub width: i32,
    pub height: i32,
    pub cell_size: f32,
}

impl Grid {
    pub fn new(width: i32, height: i32, cell_size: f32) -> Self {
        Self {
            width,
            height,
            cell_size,
        }
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
        let offset_x = -(self.width as f32 * self.cell_size) / 2.0 + self.cell_size / 2.0;
        let offset_y = -(self.height as f32 * self.cell_size) / 2.0 + self.cell_size / 2.0;
        
        let grid_x = ((world_position.x - offset_x + self.cell_size / 2.0) / self.cell_size) as i32;
        let grid_y = ((world_position.y - offset_y + self.cell_size / 2.0) / self.cell_size) as i32;
        
        if grid_x >= 0 && grid_x < self.width as i32 && grid_y >= 0 && grid_y < self.height as i32 {
            let world_x = offset_x + grid_x as f32 * self.cell_size;
            let world_y = offset_y + grid_y as f32 * self.cell_size;
            
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
        let offset_x = -(self.width as f32 * self.cell_size) / 2.0 + self.cell_size / 2.0;
        let offset_y = -(self.height as f32 * self.cell_size) / 2.0 + self.cell_size / 2.0;
        
        let world_x = offset_x + grid_x as f32 * self.cell_size;
        let world_y = offset_y + grid_y as f32 * self.cell_size;
        
        Vec2::new(world_x, world_y)
    }
}

pub fn spawn_grid(mut commands: Commands) {
    let grid = Grid::new(15, 15, 64.0);
    
    let grid_line_width = 2.0;
    let cell_visual_size = grid.cell_size - grid_line_width;
    
    let offset_x = -(grid.width as f32 * grid.cell_size) / 2.0 + grid.cell_size / 2.0;
    let offset_y = -(grid.height as f32 * grid.cell_size) / 2.0 + grid.cell_size / 2.0;
    
    commands.spawn(Sprite::from_color(Color::BLACK, Vec2::new(
        grid.width as f32 * grid.cell_size, 
        grid.height as f32 * grid.cell_size
    )))
    .insert(Transform::from_xyz(0.0, 0.0, -1.0));
    
    for y in 0..grid.height {
        for x in 0..grid.width {
            let pos_x = offset_x + x as f32 * grid.cell_size;
            let pos_y = offset_y + y as f32 * grid.cell_size;
            
            commands.spawn(Sprite::from_color(Color::WHITE, Vec2::new(cell_visual_size, cell_visual_size)))
                .insert(Transform::from_xyz(pos_x, pos_y, 0.0))
                .insert(GridCell { position: Position { x, y }, children: CellChildren(Vec::new()) });
        }
    }
    
    commands.insert_resource(grid);
}
