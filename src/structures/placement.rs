use bevy::prelude::*;
use crate::{grid::{CellChildren, ExpandGridEvent, Grid, Layer, Position}, structures::{Hub, Inventory}, ui::SelectedBuilding};
use super::{construction::{spawn_building, Building, BuildingRegistry}, NetworkChangedEvent, PlaceBuildingValidationEvent};

pub const BUILDING_LAYER: i32 = 1;

#[derive(Event, Clone)]
pub struct PlaceBuildingRequestEvent {
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
    NotEnoughResources,
}

pub fn handle_building_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grid: Res<Grid>,
    selected_building: Res<SelectedBuilding>,
    mut place_events: EventWriter<PlaceBuildingRequestEvent>,
    mut remove_events: EventWriter<RemoveBuildingEvent>,
) {
    let Some(coords) = grid.get_cursor_grid_coordinates(&windows, &camera_q) else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(building_name) = &selected_building.building_name {
            place_events.send(PlaceBuildingRequestEvent {
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
    mut validation_events: EventReader<PlaceBuildingValidationEvent>,
    grid: Res<Grid>,
    registry: Res<BuildingRegistry>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    mut central_inventory: Query<&mut Inventory, With<Hub>>,
    mut expand_events: EventWriter<ExpandGridEvent>,
    mut network_events: EventWriter<NetworkChangedEvent>,
) {
    for event in validation_events.read() {
        match &event.result {
            Ok(()) => {
                let Some((_, _, mut cell_children)) = grid_cells
                    .iter_mut()
                    .find(|(_, pos, _)| pos.x == event.request.grid_x && pos.y == event.request.grid_y) else {
                    continue;
                };
                let world_pos = grid.grid_to_world_coordinates(event.request.grid_x, event.request.grid_y);

                let (building_entity, view_radius) = spawn_building(&mut commands, &registry, &event.request.building_name, event.request.grid_x, event.request.grid_y, world_pos);

                cell_children.0.push(building_entity);

                if view_radius > 0 {
                    expand_events.send(ExpandGridEvent {
                        center_x: event.request.grid_x,
                        center_y: event.request.grid_y,
                        radius: view_radius,
                    });
                }

                // Deduct construction cost from central inventory
                if let Some(def) = registry.get_definition(&event.request.building_name) {
                    if let Some(construction_cost) = def.construction_cost {
                        if let Ok(mut inventory) = central_inventory.get_single_mut() {
                            inventory.remove_item(0, construction_cost as u32); // 0 is ore ID
                        }
                    }
                }

                network_events.send(NetworkChangedEvent);

                println!("Placed building '{}' at ({}, {})", event.request.building_name, event.request.grid_x, event.request.grid_y);
            }
            Err(error) => {
                let message = match error {
                    PlacementError::CellNotFound => "Cannot place building outside grid bounds!",
                    PlacementError::CellOccupied => "Cell is already occupied!",
                    PlacementError::NotAdjacentToNetwork => "Building must be placed adjacent to hub or connector!",
                    PlacementError::RequiresResourceNode => "Building requires resource node!",
                    PlacementError::NotEnoughResources => "Not enough resources to place building!",
                };
                println!("{}", message);
            }
        }
    }
}

pub fn remove_building(
    mut commands: Commands,
    mut remove_events: EventReader<RemoveBuildingEvent>,
    mut network_events: EventWriter<NetworkChangedEvent>,
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
                        commands.entity(building_entity).despawn_recursive();
                        to_remove.push(index);
                    }
                }
            }
        }

        for &index in to_remove.iter().rev() {
            cell_children.0.remove(index);
        }

        network_events.send(NetworkChangedEvent);
    }
}
