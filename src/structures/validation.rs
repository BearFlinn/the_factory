use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::{
    grid::{CellChildren, Layer, Position}, 
    resources::ResourceNode, 
    structures::{construction::{building_config::BuildingRegistry}, Hub, PlaceBuildingRequestEvent, BUILDING_LAYER},
    items::Inventory,
    systems::{ComputeGrid, NetworkConnectivity}
};

#[derive(Event)]
pub struct PlaceBuildingValidationEvent {
    pub result: Result<(), PlacementError>,
    pub request: PlaceBuildingRequestEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementRule {
    AdjacentToNetwork,
    RequiresResource,
}

#[derive(Debug)]
pub enum PlacementError {
    CellNotFound,
    CellOccupied,
    NotAdjacentToNetwork,
    RequiresResourceNode,
    NotEnoughResources,
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlacementError::CellNotFound => write!(f, "Cannot place building outside grid bounds!"),
            PlacementError::CellOccupied => write!(f, "Cell is already occupied!"),
            PlacementError::NotAdjacentToNetwork => write!(f, "Building must be placed adjacent to hub or connector!"),
            PlacementError::RequiresResourceNode => write!(f, "Building requires resource node!"),
            PlacementError::NotEnoughResources => write!(f, "Not enough resources to place building!"),
        }
    }
}

pub fn validate_placement(
    mut place_request: EventReader<PlaceBuildingRequestEvent>,
    mut validation_events: EventWriter<PlaceBuildingValidationEvent>,
    registry: Res<BuildingRegistry>,
    grid_cells: Query<(Entity, &Position, &CellChildren)>,
    building_layers: Query<&Layer>,
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
            if let Ok((layer)) = building_layers.get(entity) {
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

        if let Some(definition) = registry.get_definition(event.building_id) {
            // Check ore cost against central inventory
            if let Some(cost) = &definition.placement.cost {
                if let Some(inv) = inventory {
                    if !inv.has_item(0, cost.ore) { // 0 is ore ID
                        validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotEnoughResources), request: event.clone() });
                        continue;
                    }
                } else {
                    validation_events.send(PlaceBuildingValidationEvent { result: Err(PlacementError::NotEnoughResources), request: event.clone() });
                    continue;
                }
            }
            
            for rule in &definition.placement.rules {
                match rule {
                    PlacementRule::RequiresResource => {
                        // Your existing resource validation logic
                        let has_resource = cell_children.0.iter()
                            .any(|&entity| resources.contains(entity));
                        if !has_resource {
                            validation_events.send(PlaceBuildingValidationEvent { 
                                result: Err(PlacementError::RequiresResourceNode), 
                                request: event.clone() 
                            });
                            continue;
                        }
                    },
                    PlacementRule::AdjacentToNetwork => {
                        // Your existing network validation logic
                        if !network_connectivity.is_adjacent_to_core_network(event.grid_x, event.grid_y) {
                            validation_events.send(PlaceBuildingValidationEvent { 
                                result: Err(PlacementError::NotAdjacentToNetwork), 
                                request: event.clone() 
                            });
                            continue;
                        }
                    },
                }
            }
        }
        validation_events.send(PlaceBuildingValidationEvent { result: Ok(()), request: event.clone() });
    }
}
