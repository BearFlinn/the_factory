use bevy::prelude::*;
use crate::{
    grid::{CellChildren, Grid, Layer, Position}, 
    structures::{Building, BuildingComponentDef, BuildingCost, BuildingRegistry, ConstructionMaterialRequest, ConstructionSite, ConstructionSiteBundle, NetWorkComponent, PlaceBuildingValidationEvent}, 
    systems::NetworkChangedEvent, ui::SelectedBuilding, workers::Priority
};

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

pub fn handle_building_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grid: Res<Grid>,
    selected_building: Res<SelectedBuilding>,
    ui_interactions: Query<&Interaction, With<Button>>,
    mut place_events: EventWriter<PlaceBuildingRequestEvent>,
    mut remove_events: EventWriter<RemoveBuildingEvent>,
) {
    let ui_active = ui_interactions.iter().any(|interaction| {
        matches!(interaction, Interaction::Pressed | Interaction::Hovered)
    });
    
    if ui_active {
        return;
    }

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
    mut network_events: EventWriter<NetworkChangedEvent>,
    mut construction_request_events: EventWriter<ConstructionMaterialRequest>,
) {
    for event in validation_events.read() {
        if event.result.is_ok() {
            let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == event.request.grid_x && pos.y == event.request.grid_y) else {
                continue;
            };
            let world_pos = grid.grid_to_world_coordinates(event.request.grid_x, event.request.grid_y);

            if let Some(def) = registry.get_definition(&event.request.building_name) {
                let building_cost = BuildingCost { 
                    cost: def.placement.cost.to_recipe_def() 
                };
                
                let position = Position { 
                    x: event.request.grid_x, 
                    y: event.request.grid_y 
                };

                // Create construction site instead of building
                let construction_site_entity = commands.spawn(
                    ConstructionSiteBundle::new(
                        event.request.building_name.clone(),
                        building_cost.clone(),
                        position,
                        world_pos,
                        &def.appearance,
                    )
                ).id();

                if def.components.iter().any(|comp| matches!(comp, BuildingComponentDef::NetWorkComponent { .. })) {
                    commands.entity(construction_site_entity).insert(NetWorkComponent);
                }

                cell_children.0.push(construction_site_entity);

                // Request materials for construction
                construction_request_events.send(ConstructionMaterialRequest {
                    site: construction_site_entity,
                    position,
                    needed_materials: building_cost.cost.inputs.clone(),
                    priority: Priority::Medium,
                });

                network_events.send(NetworkChangedEvent);
            }
        }
    }
}

pub fn remove_building(
    mut commands: Commands,
    mut remove_events: EventReader<RemoveBuildingEvent>,
    mut network_events: EventWriter<NetworkChangedEvent>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    building_layers: Query<&Layer, Or<(With<Building>, With<ConstructionSite>)>>,
    building_positions: Query<&Position, Or<(With<Building>, With<ConstructionSite>)>>,
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
