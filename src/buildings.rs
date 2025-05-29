use bevy::prelude::*;
use crate::grid::{CellChildren, Grid, Position};
use crate::ui::SelectedBuildingType;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BuildingType {
    Harvester,
    Connector,
    Hub   
}

impl BuildingType {
    pub fn to_string(&self) -> String { 
        match self {
            BuildingType::Harvester => "Harvester".to_string(),
            BuildingType::Connector => "Connector".to_string(),
            BuildingType::Hub => "Hub".to_string(),
        }
     }
}

#[derive(Component)]
pub struct BuildingTypeComponent(pub BuildingType);

#[derive(Resource, Default)]
pub struct TotalProduction {
    pub value: u32,
}

#[derive(Resource, Default)]
pub struct HubExists {
    pub exists: bool,
    pub position: Option<(i32, i32)>,
}

#[derive(Component)]
pub struct Building;

#[derive(Component)]
pub struct Producer {
    amount: u32,
    timer: Timer,
}

#[derive(Component)]
pub struct MultiCellBuilding {
    pub width: i32,
    pub height: i32,
    pub center_x: i32,
    pub center_y: i32,
}

#[derive(Bundle)]
pub struct Harvester {
    building: Building,
    position: Position,
    producer: Producer
}

#[derive(Bundle)]
pub struct Connector {
    building: Building,
    position: Position
}

#[derive(Bundle)]
pub struct Hub {
    building: Building,
    position: Position,
    multi_cell: MultiCellBuilding,
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(TotalProduction::default());
    commands.insert_resource(HubExists::default());
}

fn is_area_clear(
    grid_cells: &Query<(Entity, &Position, &mut CellChildren)>,
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
) -> bool {
    let half_width = width / 2;
    let half_height = height / 2;
    
    for dy in -half_height..=half_height {
        for dx in -half_width..=half_width {
            let check_x = center_x + dx;
            let check_y = center_y + dy;
            
            if let Some((_, _, cell_children)) = grid_cells
                .iter()
                .find(|(_, pos, _)| pos.x == check_x && pos.y == check_y) 
            {
                if !cell_children.0.is_empty() {
                    return false;
                }
            } else {
                return false;
            }
        }
    }
    true
}

fn occupy_area(
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

fn is_adjacent_to_hub_or_connector(
    grid_cells: &Query<(Entity, &Position, &mut CellChildren)>,
    buildings: &Query<&BuildingTypeComponent>,
    target_x: i32,
    target_y: i32,
) -> bool {
    let adjacent_positions = [
        (target_x, target_y + 1), 
        (target_x, target_y - 1), 
        (target_x - 1, target_y), 
        (target_x + 1, target_y), 
    ];
    
    for (check_x, check_y) in adjacent_positions {
        if let Some((_, _, cell_children)) = grid_cells
            .iter()
            .find(|(_, pos, _)| pos.x == check_x && pos.y == check_y)
        {
            for &building_entity in &cell_children.0 {
                if let Ok(building_type_comp) = buildings.get(building_entity) {
                    match building_type_comp.0 {
                        BuildingType::Hub | BuildingType::Connector => {
                            return true; 
                        }
                        _ => {} 
                    }
                }
            }
        }
    }
    
    false 
}

pub fn place_selected_building(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grid: Res<Grid>,
    selected_building: Res<SelectedBuildingType>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    mut hub_exists: ResMut<HubExists>,
    buildings: Query<&BuildingTypeComponent>
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(building_type) = selected_building.building_type else {
        println!("No building type selected!");
        return;
    };

    if building_type == BuildingType::Hub {
        if hub_exists.exists {
            println!("Hub already exists! Only one hub is allowed.");
            return;
        }

        let center_x = grid.width / 2;
        let center_y = grid.height / 2;
        
        if !is_area_clear(&grid_cells, center_x, center_y, 3, 3) {
            println!("Cannot place hub: center area is occupied!");
            return;
        }

        let world_pos = grid.grid_to_world_coordinates(center_x, center_y);

        let building_entity = commands.spawn(Hub {
            building: Building,
            position: Position { x: center_x, y: center_y },
            multi_cell: MultiCellBuilding { 
                width: 3, 
                height: 3, 
                center_x, 
                center_y 
            },
        })
        .insert(BuildingTypeComponent(BuildingType::Hub))
        .insert(Sprite::from_color(Color::srgb(0.3, 0.3, 0.7), Vec2::new(120.0, 120.0))) // 3x32 = 96
        .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
        .id();

        occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);

        hub_exists.exists = true;
        hub_exists.position = Some((center_x, center_y));

        println!("Placed Hub at grid center: ({}, {})", center_x, center_y);
        return;
    }

    let Some(coords) = grid.get_cursor_grid_coordinates(&windows, &camera_q) else {
        println!("Cannot place building outside grid bounds!");
        return;
    };

    if building_type == BuildingType::Connector || building_type == BuildingType::Harvester {
        if !is_adjacent_to_hub_or_connector(&grid_cells, &buildings, coords.grid_x, coords.grid_y) {
            println!("Connectors and harvesters can only be placed adjacent to the hub or other connectors!");
            return;
        }
    }

    let Some((cell_entity, _, mut cell_children)) = grid_cells
        .iter_mut()
        .find(|(_, pos, _)| pos.x == coords.grid_x && pos.y == coords.grid_y) else {
        println!("Could not find grid cell at position ({}, {})", coords.grid_x, coords.grid_y);
        return;
    };

    if !cell_children.0.is_empty() {
        println!("Cell ({}, {}) is already occupied!", coords.grid_x, coords.grid_y);
        return;
    }

    let building_entity = match building_type {
        BuildingType::Harvester => {
            commands.spawn(Harvester {
                building: Building,
                position: Position { x: coords.grid_x, y: coords.grid_y },
                producer: Producer { 
                    amount: 1, 
                    timer: Timer::from_seconds(1.0, TimerMode::Repeating) 
                },
            })
            .insert(BuildingTypeComponent(BuildingType::Harvester))
            .insert(Sprite::from_color(Color::srgb(0.3, 0.7, 0.3), Vec2::new(32.0, 32.0)))
            .insert(Transform::from_xyz(coords.world_x, coords.world_y, 1.0))
            .id()
        }
        BuildingType::Connector => {
            commands.spawn(Connector {
                building: Building,
                position: Position { x: coords.grid_x, y: coords.grid_y },
            })
            .insert(BuildingTypeComponent(BuildingType::Connector))
            .insert(Sprite::from_color(Color::srgb(0.7, 0.3, 0.7), Vec2::new(16.0, 16.0)))
            .insert(Transform::from_xyz(coords.world_x, coords.world_y, 1.0))
            .id()
        }
        BuildingType::Hub => unreachable!(),
    };

    cell_children.0.push(building_entity);
    
    println!("Placed {:?} at grid cell: ({}, {}), added to cell entity {:?}", 
        building_type, coords.grid_x, coords.grid_y, cell_entity);
}

pub fn update_producers(
    mut query: Query<&mut Producer>,
    mut total_production: ResMut<TotalProduction>,
    time: Res<Time>,
) {
    for mut producer in query.iter_mut() {
        if producer.timer.tick(time.delta()).just_finished() {
            total_production.value += producer.amount;
            producer.timer.reset();
        }
    }
}

pub struct BuildingsPlugin;
impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, (place_selected_building, update_producers));
    }
}