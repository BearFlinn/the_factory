use bevy::prelude::*;
use crate::{
    grid::{CellChildren, Layer, Position}, resources::RawMaterial, structures::construction::{BuildingRegistry, BuildingType, Hub, MultiCellBuilding}
};

use std::collections::{HashSet, VecDeque};

use super::{PlaceBuildingRequestEvent, PlacementError, TotalProduction, BUILDING_LAYER};

#[derive(Event)]
pub struct PlaceBuildingValidationEvent {
    pub result: Result<(), PlacementError>,
    pub request: PlaceBuildingRequestEvent,
}

pub fn validate_placement(
    mut place_request: EventReader<PlaceBuildingRequestEvent>,
    mut validation_events: EventWriter<PlaceBuildingValidationEvent>,
    registry: Res<BuildingRegistry>,
    grid_cells: Query<(Entity, &Position, &CellChildren)>,
    building_layers: Query<(&BuildingType, &Layer)>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
    available_production: Res<TotalProduction>,
    resources: Query<&RawMaterial>,
)  {
    for event in place_request.read() {
        // Check if cell exists
        let Some((_, _, cell_children)) = grid_cells
            .iter()
            .find(|(_, pos, _)| pos.x == event.grid_x && pos.y == event.grid_y) else {
            validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::CellNotFound), request: event.clone() });
            continue;
        };

        // Check if cell is occupied
        let mut cell_occupied = false;
        for &entity in &cell_children.0 {
            if let Ok((_, layer)) = building_layers.get(entity) {
                if layer.0 == BUILDING_LAYER {
                    validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::CellOccupied), request: event.clone() });
                    cell_occupied = true;
                    break;
                }
            }
        }
        if cell_occupied {
            continue;
        }

        // Check building requirements
        if let Some(definition) = registry.get_definition(&event.building_name) {
            if let Some(cost) = definition.construction_cost {
                if cost as u32 > available_production.ore {
                    validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotEnoughResources), request: event.clone() });
                    continue;
                }
            }
            
            if definition.building_type == BuildingType::Harvester {
                println!("Checking for resource at ({}, {})", event.grid_x, event.grid_y);
                println!("Cell has {} entities", cell_children.0.len());
                
                for &entity in &cell_children.0 {
                    if resources.contains(entity) {
                        println!("Found resource entity: {:?}", entity);
                    } else {
                        println!("Non-resource entity: {:?}", entity);
                    }
                }
                
                let has_resource = cell_children.0.iter()
                    .any(|&entity| resources.contains(entity));
                
                println!("Has resource: {}", has_resource);
                
                if !has_resource {
                    validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::RequiresResourceNode), request: event.clone() });
                    continue;
                }
            }
        }

        // Check adjacency
        if !is_adjacent_to_hub_or_connector(&grid_cells, &building_layers, &hub, event.grid_x, event.grid_y) {
            validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotAdjacentToNetwork), request: event.clone() });
            continue; 
        }
        validation_events.send(PlaceBuildingValidationEvent { result: Ok(()), request: event.clone() });
    }
}

pub fn is_adjacent_to_hub_or_connector(
    grid_cells: &Query<(Entity, &Position, &CellChildren)>,
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

pub fn is_connected_to_network(
    grid_cells: &Query<(Entity, &Position, &CellChildren)>,
    building_layers: &Query<(&BuildingType, &Layer)>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
    target_x: i32,
    target_y: i32,
) -> bool {
    let connected_cells = get_connected_network_cells(grid_cells, building_layers, hub);
    
    // Check if target position is adjacent to any connected network cell
    let adjacent_positions = [
        (target_x, target_y + 1),
        (target_x, target_y - 1),
        (target_x - 1, target_y),
        (target_x + 1, target_y),
    ];
    
    adjacent_positions.iter().any(|pos| connected_cells.contains(pos))
}

fn get_connected_network_cells(
    grid_cells: &Query<(Entity, &Position, &CellChildren)>,
    building_layers: &Query<(&BuildingType, &Layer)>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
) -> HashSet<(i32, i32)> {
    let mut connected_cells = HashSet::new();
    let mut queue = VecDeque::new();
    
    // Start floodfill from all hub cells
    for (multi_cell_building, _) in hub.iter() {
        let half_width = multi_cell_building.width / 2;
        let half_height = multi_cell_building.height / 2;
        
        for dy in -half_height..=half_height {
            for dx in -half_width..=half_width {
                let hub_x = multi_cell_building.center_x + dx;
                let hub_y = multi_cell_building.center_y + dy;
                let pos = (hub_x, hub_y);
                
                if connected_cells.insert(pos) {
                    queue.push_back(pos);
                }
            }
        }
    }
    
    // Floodfill through connectors
    while let Some((x, y)) = queue.pop_front() {
        let adjacent_positions = [(x+1, y), (x-1, y), (x, y+1), (x, y-1)];
        
        for (adj_x, adj_y) in adjacent_positions {
            // Skip if already visited
            if connected_cells.contains(&(adj_x, adj_y)) {
                continue;
            }
            
            // Check if this cell has a connector
            if let Some((_, _, cell_children)) = grid_cells
                .iter()
                .find(|(_, pos, _)| pos.x == adj_x && pos.y == adj_y)
            {
                for &building_entity in &cell_children.0 {
                    if let Ok((building_type, layer)) = building_layers.get(building_entity) {
                        if layer.0 == BUILDING_LAYER && *building_type == BuildingType::Connector {
                            let pos = (adj_x, adj_y);
                            if connected_cells.insert(pos) {
                                queue.push_back(pos);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    
    connected_cells
}