use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{grid::{Layer, Position}, structures::{Building, Hub, MultiCellBuilding, NetWorkComponent, BUILDING_LAYER}};

#[derive(Event)]
pub struct NetworkChangedEvent;

#[derive(Component)]
pub struct NetworkConnection {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

#[derive(Resource, Default)]
pub struct NetworkConnectivity {
    // Core network: only hubs and connectors (for placement validation and pathfinding)
    core_network_cells: HashSet<(i32, i32)>,
    // Extended network: core + buildings adjacent to core (for operational status)
    connected_cells: HashSet<(i32, i32)>,
}

impl NetworkConnectivity {
    pub fn is_cell_connected(&self, x: i32, y: i32) -> bool {
        self.connected_cells.contains(&(x, y))
    }
    
    pub fn is_core_network_cell(&self, x: i32, y: i32) -> bool {
        self.core_network_cells.contains(&(x, y))
    }
    
    pub fn is_adjacent_to_connected_network(&self, x: i32, y: i32) -> bool {
        let adjacent_positions = [(x, y + 1), (x, y - 1), (x - 1, y), (x + 1, y)];
        adjacent_positions.iter().any(|pos| self.connected_cells.contains(pos))
    }
    
    // New method for placement validation - only checks adjacency to core network
    pub fn is_adjacent_to_core_network(&self, x: i32, y: i32) -> bool {
        let adjacent_positions = [(x, y + 1), (x, y - 1), (x - 1, y), (x + 1, y)];
        adjacent_positions.iter().any(|pos| self.core_network_cells.contains(pos))
    }
}

pub fn calculate_network_connectivity(
    building_layers: &Query<(&Position, &Layer, Option<&NetWorkComponent>), With<Building>>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
) -> (HashSet<(i32, i32)>, HashSet<(i32, i32)>) {
    let mut core_network_cells = HashSet::new();
    let mut queue = VecDeque::new();
    
    // Add hub positions as starting points
    for (multi_cell, _) in hub.iter() {
        let half_width = multi_cell.width / 2;
        let half_height = multi_cell.height / 2;
        
        for dy in -half_width..=half_width {
            for dx in -half_width..=half_height {
                let pos = (multi_cell.center_x + dx, multi_cell.center_y + dy);
                if core_network_cells.insert(pos) {
                    queue.push_back(pos);
                }
            }
        }
    }
    
    // Flood fill to find all positions connected via connectors
    while let Some((x, y)) = queue.pop_front() {
        for (adj_x, adj_y) in [(x+1, y), (x-1, y), (x, y+1), (x, y-1)] {
            if core_network_cells.contains(&(adj_x, adj_y)) {
                continue;
            }
            
            // Check if this adjacent position has a connector
            let has_connector = building_layers.iter().any(|(position, layer, building)| {
                layer.0 == BUILDING_LAYER && 
                building == Some(&NetWorkComponent) &&
                position.x == adj_x && position.y == adj_y
            });
            
            if has_connector {
                core_network_cells.insert((adj_x, adj_y));
                queue.push_back((adj_x, adj_y));
            }
        }
    }
    
    // Create extended network: include all buildings adjacent to core network
    let mut connected_cells = core_network_cells.clone();
    for (position, layer, _) in building_layers.iter() {
        if layer.0 == BUILDING_LAYER {
            let building_pos = (position.x, position.y);
            
            // Skip if already in core network
            if core_network_cells.contains(&building_pos) {
                continue;
            }
            
            // Check if this building is adjacent to any cell in the core network
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let adjacent = (building_pos.0 + dx, building_pos.1 + dy);
                if core_network_cells.contains(&adjacent) {
                    connected_cells.insert(building_pos);
                    break;
                }
            }
        }
    }
    
    (core_network_cells, connected_cells)
}

pub fn update_network_connectivity(
    mut network_connectivity: ResMut<NetworkConnectivity>,
    mut network_events: EventReader<NetworkChangedEvent>,
    building_layers: Query<(&Position, &Layer, Option<&NetWorkComponent>), With<Building>>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
) {
    // Calculate on first run even without event, or when event received
    let should_update = network_events.len() > 0 || network_connectivity.core_network_cells.is_empty();
    
    if !should_update {
        return;
    }
    
    network_events.clear();
    
    let (core_network, extended_network) = calculate_network_connectivity(&building_layers, &hub);
    network_connectivity.core_network_cells = core_network;
    network_connectivity.connected_cells = extended_network;
}

pub fn update_visual_network_connections(
    mut commands: Commands,
    mut network_events: EventReader<NetworkChangedEvent>,
    network_connectivity: Res<NetworkConnectivity>,
    existing_connections: Query<Entity, With<NetworkConnection>>,
) {
    if network_events.is_empty() {
        return;
    }
    network_events.clear();
    
    // Remove old visual connections
    for entity in existing_connections.iter() {
        commands.entity(entity).despawn();
    }
    
    // Create selective visual connections
    let extended_positions: Vec<_> = network_connectivity.connected_cells.iter().cloned().collect();
    
    for i in 0..extended_positions.len() {
        for j in (i + 1)..extended_positions.len() {
            let pos1 = extended_positions[i];
            let pos2 = extended_positions[j];
            
            let dx = (pos2.0 - pos1.0).abs();
            let dy = (pos2.1 - pos1.1).abs();
            
            // Only process adjacent cells
            if !((dx == 1 && dy == 0) || (dx == 0 && dy == 1)) {
                continue;
            }
            
            // Determine cell types using core network membership
            let pos1_is_core = network_connectivity.is_core_network_cell(pos1.0, pos1.1);
            let pos2_is_core = network_connectivity.is_core_network_cell(pos2.0, pos2.1);
            
            // Draw connection if at least one endpoint is core network (hub/connector)
            // This excludes building↔building while including all core↔building connections
            if pos1_is_core || pos2_is_core {
                spawn_connection_visual(&mut commands, pos1, pos2);
            }
        }
    }
}

fn spawn_connection_visual(
    commands: &mut Commands,
    from: (i32, i32),
    to: (i32, i32),
) {
    let from_world = Vec2::new(from.0 as f32 * 64.0, from.1 as f32 * 64.0);
    let to_world = Vec2::new(to.0 as f32 * 64.0, to.1 as f32 * 64.0);
    
    let center = (from_world + to_world) / 2.0;
    let direction = to_world - from_world;
    let length = direction.length();
    let angle = direction.y.atan2(direction.x);
    
    commands.spawn((
        NetworkConnection { from, to },
        Sprite::from_color(Color::srgb(0.8, 0.8, 0.2), Vec2::new(length, 4.0)),
        Transform::from_xyz(center.x, center.y, 0.5)
            .with_rotation(Quat::from_rotation_z(angle)),
    ));
}