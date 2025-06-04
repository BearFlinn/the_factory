use bevy::prelude::*;
use crate::{
    grid::{CellChildren, Layer, Position}, resources::ResourceNode, structures::{construction::{BuildingRegistry, BuildingType}, ComputeGrid, Hub, Inventory, NetworkConnectivity}
};

use super::{PlaceBuildingRequestEvent, PlacementError, BUILDING_LAYER};

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
    central_inventory: Query<&Inventory, With<Hub>>,
    available_compute: Res<ComputeGrid>,
    resources: Query<&ResourceNode>,
    network_connectivity: Res<NetworkConnectivity>,
)  {
    let inventory = central_inventory.get_single().ok();
    
    for event in place_request.read() {
        let Some((_, _, cell_children)) = grid_cells
            .iter()
            .find(|(_, pos, _)| pos.x == event.grid_x && pos.y == event.grid_y) else {
            validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::CellNotFound), request: event.clone() });
            continue;
        };

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

        if let Some(definition) = registry.get_definition(&event.building_name) {
            // Check ore cost against central inventory
            if let Some(cost) = definition.construction_cost {
                if let Some(inv) = inventory {
                    if !inv.has_item(0, cost as u32) { // 0 is ore ID
                        validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotEnoughResources), request: event.clone() });
                        continue;
                    }
                } else {
                    validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotEnoughResources), request: event.clone() });
                    continue;
                }
            }

            if let Some(cost) = definition.compute_consumption {
                if cost > available_compute.available {
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

        if !network_connectivity.is_adjacent_to_connected_network(event.grid_x, event.grid_y) {
            validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotAdjacentToNetwork), request: event.clone() });
            continue; 
        }
        validation_events.send(PlaceBuildingValidationEvent { result: Ok(()), request: event.clone() });
    }
}
