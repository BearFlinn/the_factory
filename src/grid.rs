use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component, Clone, Copy, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct Layer(pub i32);

#[derive(Component, Clone)]
pub struct CellChildren(pub Vec<Entity>);

#[derive(Bundle)]
pub struct GridCell {
    pub position: Position,
    pub children: CellChildren,
}

#[derive(Debug, Clone, Copy)]
pub struct GridCoordinates {
    pub grid_x: i32,
    pub grid_y: i32,
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

    pub fn get_cursor_grid_coordinates(
        &self,
        windows: &Query<&Window>,
        camera_q: &Query<(&Camera, &GlobalTransform)>,
    ) -> Option<GridCoordinates> {
        let window = windows.single();
        let (camera, camera_transform) = camera_q.single();

        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
            .map(|ray| ray.origin.truncate())
        {
            self.world_to_grid_coordinates(world_position)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn world_to_grid_coordinates(&self, world_position: Vec2) -> Option<GridCoordinates> {
        let grid_x = (world_position.x / self.cell_size).round() as i32;
        let grid_y = (world_position.y / self.cell_size).round() as i32;

        if self.valid_coordinates.contains(&(grid_x, grid_y)) {
            Some(GridCoordinates { grid_x, grid_y })
        } else {
            None
        }
    }

    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn grid_to_world_coordinates(&self, grid_x: i32, grid_y: i32) -> Vec2 {
        let world_x = grid_x as f32 * self.cell_size;
        let world_y = grid_y as f32 * self.cell_size;

        Vec2::new(world_x, world_y)
    }

    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn get_coordinates_in_radius(center_x: i32, center_y: i32, radius: i32) -> Vec<(i32, i32)> {
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

pub fn spawn_grid(mut commands: Commands, mut grid: ResMut<Grid>) {
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
}

#[allow(clippy::cast_precision_loss)]
pub fn spawn_cell(commands: &mut Commands, grid: &Grid, x: i32, y: i32) -> Entity {
    let grid_line_width = 2.0;
    let cell_visual_size = grid.cell_size - grid_line_width;

    let pos_x = x as f32 * grid.cell_size;
    let pos_y = y as f32 * grid.cell_size;

    let cell_entity = commands
        .spawn((
            Sprite::from_color(Color::BLACK, Vec2::new(grid.cell_size, grid.cell_size)),
            Transform::from_xyz(pos_x, pos_y, 0.0),
            GridCell {
                position: Position { x, y },
                children: CellChildren(Vec::new()),
            },
        ))
        .id();

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
        let new_coordinates =
            Grid::get_coordinates_in_radius(event.center_x, event.center_y, event.radius);
        for (x, y) in new_coordinates {
            if !grid.valid_coordinates.contains(&(x, y)) {
                grid.add_coordinate(x, y);
                spawn_cell(&mut commands, &grid, x, y);
                cell_event.send(NewCellEvent { x, y });
            }
        }
    }
}

#[derive(Event)]
pub struct ExpandGridCellsEvent {
    pub coordinates: Vec<(i32, i32)>,
}

pub fn handle_grid_cells_expansion(
    mut commands: Commands,
    mut expand_events: EventReader<ExpandGridCellsEvent>,
    mut grid: ResMut<Grid>,
    mut cell_event: EventWriter<NewCellEvent>,
) {
    for event in expand_events.read() {
        for (x, y) in &event.coordinates {
            if !grid.valid_coordinates.contains(&(*x, *y)) {
                grid.add_coordinate(*x, *y);
                spawn_cell(&mut commands, &grid, *x, *y);
                cell_event.send(NewCellEvent { x: *x, y: *y });
            }
        }
    }
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NewCellEvent>()
            .add_event::<ExpandGridEvent>()
            .add_event::<ExpandGridCellsEvent>()
            .add_systems(Startup, (setup_grid, spawn_grid).chain())
            .add_systems(Update, (handle_grid_expansion, handle_grid_cells_expansion));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::float_cmp)]

    use super::*;

    const DEFAULT_CELL_SIZE: f32 = 64.0;

    #[test]
    fn new_creates_grid_with_empty_coordinates() {
        let grid = Grid::new(DEFAULT_CELL_SIZE);

        assert_eq!(grid.cell_size, DEFAULT_CELL_SIZE);
        assert!(grid.valid_coordinates.is_empty());
    }

    #[test]
    fn add_coordinate_returns_true_when_new() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);

        let result = grid.add_coordinate(5, 10);

        assert!(result);
        assert!(grid.valid_coordinates.contains(&(5, 10)));
    }

    #[test]
    fn add_coordinate_returns_false_when_duplicate() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(5, 10);

        let result = grid.add_coordinate(5, 10);

        assert!(!result);
        assert_eq!(grid.valid_coordinates.len(), 1);
    }

    #[test]
    fn world_to_grid_coordinates_center_position() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(0, 0);

        let result = grid.world_to_grid_coordinates(Vec2::new(0.0, 0.0));

        assert!(result.is_some());
        let coords = result.unwrap();
        assert_eq!(coords.grid_x, 0);
        assert_eq!(coords.grid_y, 0);
    }

    #[test]
    fn world_to_grid_coordinates_positive_coordinates() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(2, 3);

        // Position at center of cell (2, 3) = (128.0, 192.0)
        let result = grid.world_to_grid_coordinates(Vec2::new(128.0, 192.0));

        assert!(result.is_some());
        let coords = result.unwrap();
        assert_eq!(coords.grid_x, 2);
        assert_eq!(coords.grid_y, 3);
    }

    #[test]
    fn world_to_grid_coordinates_negative_coordinates() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(-3, -2);

        // Position at center of cell (-3, -2) = (-192.0, -128.0)
        let result = grid.world_to_grid_coordinates(Vec2::new(-192.0, -128.0));

        assert!(result.is_some());
        let coords = result.unwrap();
        assert_eq!(coords.grid_x, -3);
        assert_eq!(coords.grid_y, -2);
    }

    #[test]
    fn world_to_grid_coordinates_returns_none_for_invalid_cell() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(0, 0);

        // Position at cell (1, 1) which is not in valid_coordinates
        let result = grid.world_to_grid_coordinates(Vec2::new(64.0, 64.0));

        assert!(result.is_none());
    }

    #[test]
    fn world_to_grid_coordinates_rounds_to_nearest_cell() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        grid.add_coordinate(1, 1);

        // Position slightly off center but should round to (1, 1)
        let result = grid.world_to_grid_coordinates(Vec2::new(70.0, 60.0));

        assert!(result.is_some());
        let coords = result.unwrap();
        assert_eq!(coords.grid_x, 1);
        assert_eq!(coords.grid_y, 1);
    }

    #[test]
    fn grid_to_world_coordinates_origin() {
        let grid = Grid::new(DEFAULT_CELL_SIZE);

        let result = grid.grid_to_world_coordinates(0, 0);

        assert_eq!(result, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn grid_to_world_coordinates_positive_quadrant() {
        let grid = Grid::new(DEFAULT_CELL_SIZE);

        let result = grid.grid_to_world_coordinates(3, 5);

        assert_eq!(result, Vec2::new(192.0, 320.0));
    }

    #[test]
    fn grid_to_world_coordinates_negative_quadrant() {
        let grid = Grid::new(DEFAULT_CELL_SIZE);

        let result = grid.grid_to_world_coordinates(-2, -4);

        assert_eq!(result, Vec2::new(-128.0, -256.0));
    }

    #[test]
    fn grid_to_world_coordinates_mixed_coordinates() {
        let grid = Grid::new(DEFAULT_CELL_SIZE);

        let result = grid.grid_to_world_coordinates(-3, 7);

        assert_eq!(result, Vec2::new(-192.0, 448.0));
    }

    #[test]
    fn get_coordinates_in_radius_zero_radius() {
        let coords = Grid::get_coordinates_in_radius(5, 5, 0);

        assert_eq!(coords.len(), 1);
        assert!(coords.contains(&(5, 5)));
    }

    #[test]
    fn get_coordinates_in_radius_one() {
        let coords = Grid::get_coordinates_in_radius(0, 0, 1);

        // Radius 1 with circular distance check: center + 4 cardinal neighbors
        // (0,0), (1,0), (-1,0), (0,1), (0,-1)
        assert_eq!(coords.len(), 5);
        assert!(coords.contains(&(0, 0)));
        assert!(coords.contains(&(1, 0)));
        assert!(coords.contains(&(-1, 0)));
        assert!(coords.contains(&(0, 1)));
        assert!(coords.contains(&(0, -1)));
    }

    #[test]
    fn get_coordinates_in_radius_three() {
        let coords = Grid::get_coordinates_in_radius(0, 0, 3);

        // Check center is included
        assert!(coords.contains(&(0, 0)));

        // Check cardinal directions at max distance are included
        assert!(coords.contains(&(3, 0)));
        assert!(coords.contains(&(-3, 0)));
        assert!(coords.contains(&(0, 3)));
        assert!(coords.contains(&(0, -3)));

        // Corner cells at (3, 3) should NOT be included (distance ~4.24 > 3)
        assert!(!coords.contains(&(3, 3)));
        assert!(!coords.contains(&(-3, -3)));

        // Cells at (2, 2) SHOULD be included (distance ~2.83 <= 3)
        assert!(coords.contains(&(2, 2)));
    }

    #[test]
    fn bidirectional_conversion_consistency() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        let original_x = 7;
        let original_y = -4;
        grid.add_coordinate(original_x, original_y);

        let world_pos = grid.grid_to_world_coordinates(original_x, original_y);
        let grid_coords = grid.world_to_grid_coordinates(world_pos);

        assert!(grid_coords.is_some());
        let coords = grid_coords.unwrap();
        assert_eq!(coords.grid_x, original_x);
        assert_eq!(coords.grid_y, original_y);
    }

    #[test]
    fn bidirectional_conversion_consistency_large_values() {
        let mut grid = Grid::new(DEFAULT_CELL_SIZE);
        let original_x = 1000;
        let original_y = -500;
        grid.add_coordinate(original_x, original_y);

        let world_pos = grid.grid_to_world_coordinates(original_x, original_y);
        let grid_coords = grid.world_to_grid_coordinates(world_pos);

        assert!(grid_coords.is_some());
        let coords = grid_coords.unwrap();
        assert_eq!(coords.grid_x, original_x);
        assert_eq!(coords.grid_y, original_y);
    }
}
