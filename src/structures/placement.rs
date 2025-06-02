use bevy::prelude::*;
use crate::{grid::{CellChildren, ExpandGridEvent, Grid, Layer, Position}, resources::RawMaterial, ui::SelectedBuilding};
use super::construction::{Building, BuildingType, Hub, MultiCellBuilding, spawn_building, BuildingRegistry};

const BUILDING_LAYER: i32 = 1;

#[derive(Event)]
pub struct PlaceBuildingEvent {
    pub building_name: String,
    pub grid_x: i32,
    pub grid_y: i32,
}

#[derive(Event)]
pub struct RemoveBuildingEvent {
    pub grid_x: i32,
    pub grid_y: i32,
}

#[derive(Debug)]
pub enum PlacementError {
    CellNotFound,
    CellOccupied,
    NotAdjacentToNetwork,
    RequiresResourceNode,
}

pub fn handle_building_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grid: Res<Grid>,
    selected_building: Res<SelectedBuilding>,
    mut place_events: EventWriter<PlaceBuildingEvent>,
    mut remove_events: EventWriter<RemoveBuildingEvent>,
) {
    let Some(coords) = grid.get_cursor_grid_coordinates(&windows, &camera_q) else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(building_name) = &selected_building.building_name {
            place_events.send(PlaceBuildingEvent {
                building_name: building_name.clone(),
                grid_x: coords.grid_x,
                grid_y: coords.grid_y,
            });
        }
    }

    if mouse_button.just_pressed(MouseButton::Right) {
        remove_events.send(RemoveBuildingEvent {
            grid_x: coords.grid_x,
            grid_y: coords.grid_y,
        });
    }
}

pub fn place_building(
    mut commands: Commands,
    mut place_events: EventReader<PlaceBuildingEvent>,
    grid: Res<Grid>,
    registry: Res<BuildingRegistry>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    building_layers: Query<(&BuildingType, &Layer)>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
    resources: Query<&RawMaterial>,
    mut expand_events: EventWriter<ExpandGridEvent>,
) {
    for event in place_events.read() {
        match validate_placement(
            event.grid_x, 
            event.grid_y, 
            &event.building_name,
            &registry,
            &grid_cells, 
            &building_layers, 
            &hub,
            &resources,
        ) {
            Ok(()) => {
                let Some((_, _, mut cell_children)) = grid_cells
                    .iter_mut()
                    .find(|(_, pos, _)| pos.x == event.grid_x && pos.y == event.grid_y) else {
                    continue;
                };

                let world_pos = grid.grid_to_world_coordinates(event.grid_x, event.grid_y);

                let (building_entity, view_radius) = spawn_building(&mut commands, &registry, &event.building_name, event.grid_x, event.grid_y, world_pos);

                cell_children.0.push(building_entity);

                expand_events.send(ExpandGridEvent {
                    center_x: event.grid_x,
                    center_y: event.grid_y,
                    radius: view_radius,
                });

                println!("Placed {} at ({}, {})", event.building_name, event.grid_x, event.grid_y);
            }
            Err(error) => {
                let message = match error {
                    PlacementError::CellNotFound => "Cannot place building outside grid bounds!",
                    PlacementError::CellOccupied => "Cell is already occupied!",
                    PlacementError::NotAdjacentToNetwork => "Building must be placed adjacent to hub or connector!",
                    PlacementError::RequiresResourceNode => "Building requires a resource node!",
                };
                println!("{}", message);
            }
        }
    }
}

pub fn remove_building(
    mut commands: Commands,
    mut remove_events: EventReader<RemoveBuildingEvent>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    building_layers: Query<&Layer, With<Building>>,
    building_positions: Query<&Position, With<Building>>,
) {
    for event in remove_events.read() {
        let Some((_, _, mut cell_children)) = grid_cells
            .iter_mut()
            .find(|(_, pos, _)| pos.x == event.grid_x && pos.y == event.grid_y)
            else {
            continue;
        };

        let mut to_remove = Vec::new();

        for (index, &building_entity) in cell_children.0.iter().enumerate() {
            if building_layers.contains(building_entity) {
                if let Ok(pos) = building_positions.get(building_entity) {
                    if pos.x == event.grid_x && pos.y == event.grid_y {
                        commands.entity(building_entity).despawn();
                        to_remove.push(index);
                    }
                }
            }
        }

        for &index in to_remove.iter().rev() {
            cell_children.0.remove(index);
        }
    }
}

fn validate_placement(
    grid_x: i32,
    grid_y: i32,
    building_name: &str,
    registry: &BuildingRegistry,
    grid_cells: &Query<(Entity, &Position, &mut CellChildren)>,
    building_layers: &Query<(&BuildingType, &Layer)>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
    resources: &Query<&crate::resources::RawMaterial>,
) -> Result<(), PlacementError> {
    let Some((_, _, cell_children)) = grid_cells
        .iter()
        .find(|(_, pos, _)| pos.x == grid_x && pos.y == grid_y) else {
        return Err(PlacementError::CellNotFound);
    };

    for &entity in &cell_children.0 {
        if let Ok((_, layer)) = building_layers.get(entity) {
            if layer.0 == BUILDING_LAYER {
                return Err(PlacementError::CellOccupied);
            }
        }
    }

    if let Some(definition) = registry.get_definition(building_name) {
        if definition.building_type == BuildingType::Harvester {
            let has_resource = cell_children.0.iter()
                .any(|&entity| resources.contains(entity));
            
            if !has_resource {
                return Err(PlacementError::RequiresResourceNode);
            }
        }
    }

    if !is_adjacent_to_hub_or_connector(grid_cells, building_layers, hub, grid_x, grid_y) {
        return Err(PlacementError::NotAdjacentToNetwork);
    } else {
        return Ok(());
    }
}

fn is_adjacent_to_hub_or_connector(
    grid_cells: &Query<(Entity, &Position, &mut CellChildren)>,
    building_layers: &Query<(&BuildingType, &Layer)>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
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
                if let Ok((building_type, layer)) = building_layers.get(building_entity) {
                    if layer.0 == BUILDING_LAYER && *building_type == BuildingType::Connector {
                        return true;
                    }
                }
            }
        }
    }

    for (multi_cell_building, _) in hub.iter() {
        let half_width = multi_cell_building.width / 2;
        let half_height = multi_cell_building.height / 2;

        let hub_min_x = multi_cell_building.center_x - half_width;
        let hub_max_x = multi_cell_building.center_x + half_width;
        let hub_min_y = multi_cell_building.center_y - half_height;
        let hub_max_y = multi_cell_building.center_y + half_height;

        let expanded_min_x = hub_min_x - 1;
        let expanded_max_x = hub_max_x + 1;
        let expanded_min_y = hub_min_y - 1;
        let expanded_max_y = hub_max_y + 1;

        let within_expanded = target_x >= expanded_min_x && target_x <= expanded_max_x &&
                             target_y >= expanded_min_y && target_y <= expanded_max_y;

        let inside_hub = target_x >= hub_min_x && target_x <= hub_max_x &&
                        target_y >= hub_min_y && target_y <= hub_max_y;

        if within_expanded && !inside_hub {
            return true;
        }
    }

    false
}
