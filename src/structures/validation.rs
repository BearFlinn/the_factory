use crate::{
    grid::{CellChildren, Layer, Position},
    resources::ResourceNode,
    structures::{
        construction::building_config::BuildingRegistry, PlaceBuildingRequestEvent, BUILDING_LAYER,
    },
    systems::NetworkConnectivity,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

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
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlacementError::CellNotFound => write!(f, "Cannot place building outside grid bounds!"),
            PlacementError::CellOccupied => write!(f, "Cell is already occupied!"),
            PlacementError::NotAdjacentToNetwork => {
                write!(f, "Building must be placed adjacent to hub or connector!")
            }
            PlacementError::RequiresResourceNode => write!(f, "Building requires resource node!"),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn validate_placement(
    mut place_request: EventReader<PlaceBuildingRequestEvent>,
    mut validation_events: EventWriter<PlaceBuildingValidationEvent>,
    registry: Res<BuildingRegistry>,
    grid_cells: Query<(Entity, &Position, &CellChildren)>,
    building_layers: Query<&Layer>,
    resources: Query<&ResourceNode>,
    network_connectivity: Res<NetworkConnectivity>,
) {
    'event_loop: for event in place_request.read() {
        let Some((_, _, cell_children)) = grid_cells
            .iter()
            .find(|(_, pos, _)| pos.x == event.grid_x && pos.y == event.grid_y)
        else {
            validation_events.send(PlaceBuildingValidationEvent {
                result: Err(PlacementError::CellNotFound),
                request: event.clone(),
            });
            continue 'event_loop;
        };

        // Check if cell is occupied
        for &entity in &cell_children.0 {
            if let Ok(layer) = building_layers.get(entity) {
                if layer.0 == BUILDING_LAYER {
                    validation_events.send(PlaceBuildingValidationEvent {
                        result: Err(PlacementError::CellOccupied),
                        request: event.clone(),
                    });
                    continue 'event_loop;
                }
            }
        }

        if let Some(definition) = registry.get_definition(&event.building_name) {
            // Note: Removed resource availability check - construction sites will handle material delivery

            // Validate placement rules
            for rule in &definition.placement.rules {
                match rule {
                    PlacementRule::RequiresResource => {
                        let has_resource = cell_children
                            .0
                            .iter()
                            .any(|&entity| resources.contains(entity));
                        if !has_resource {
                            validation_events.send(PlaceBuildingValidationEvent {
                                result: Err(PlacementError::RequiresResourceNode),
                                request: event.clone(),
                            });
                            continue 'event_loop;
                        }
                    }
                    PlacementRule::AdjacentToNetwork => {
                        if !network_connectivity
                            .is_adjacent_to_core_network(event.grid_x, event.grid_y)
                        {
                            validation_events.send(PlaceBuildingValidationEvent {
                                result: Err(PlacementError::NotAdjacentToNetwork),
                                request: event.clone(),
                            });
                            continue 'event_loop;
                        }
                    }
                }
            }
        }

        // All validations passed
        validation_events.send(PlaceBuildingValidationEvent {
            result: Ok(()),
            request: event.clone(),
        });
    }
}
