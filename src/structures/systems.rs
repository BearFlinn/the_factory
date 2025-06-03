use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::grid::{Layer, Position};

use super::{Building, BuildingType, Hub, MultiCellBuilding, Operational, PowerConsumer, PowerGenerator, Producer, BUILDING_LAYER};

#[derive(Resource, Default, Clone)]
pub struct TotalProduction {
    pub ore: u32,
}

#[derive(Resource, Default)]
pub struct PowerGrid {
    pub capacity: i32,
    pub usage: i32,
    pub available: i32
}

#[derive(Component)]
pub struct NetworkConnection {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

#[derive(Event)]
pub struct NetworkChangedEvent;

#[derive(Resource, Default)]
pub struct NetworkConnectivity {
    connected_cells: HashSet<(i32, i32)>,
}

impl NetworkConnectivity {
    pub fn is_adjacent_to_connected_network(&self, x: i32, y: i32) -> bool {
        let adjacent_positions = [(x, y + 1), (x, y - 1), (x - 1, y), (x + 1, y)];
        adjacent_positions.iter().any(|pos| self.connected_cells.contains(pos))
    }
}

pub fn update_power_grid(
    mut power_grid: ResMut<PowerGrid>,
    generators: Query<(&PowerGenerator, &Operational)>,
    consumers: Query<&PowerConsumer>,
) {
    let mut total_production: i32 = 0;
    for (generator, operational) in generators.iter() {
        if !operational.0 {
            continue;
        }
        
        total_production += generator.amount;
        
    }

    let total_consumption: i32 = consumers.iter().map(|c| c.amount).sum();

    power_grid.capacity = total_production;
    power_grid.usage = total_consumption;
    power_grid.available = total_production - total_consumption;
}

pub fn update_producers(
    mut query: Query<(&mut Producer, &Operational)>,
    mut total_production: ResMut<TotalProduction>,
    time: Res<Time>,
) {
    for (mut producer, operational) in query.iter_mut() {
        if !operational.0 {
            continue;
        }
        
        if producer.timer.tick(time.delta()).just_finished() {
            total_production.ore += producer.amount;
            producer.timer.reset();
        }
    }
}

pub fn update_operational_status_optimized(
    mut buildings: Query<(&BuildingType, &Position, &mut Operational, Option<&PowerConsumer>), With<Building>>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
) {
    let has_power = power_grid.available >= 0;
    
    for (building_type, pos, mut operational, power_consumer) in buildings.iter_mut() {
        if !network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y) {
            operational.0 = false;
            continue; 
        }
        
        operational.0 = match building_type {
            BuildingType::Generator => true,
            _ => {
                if power_consumer.is_some() {
                    has_power
                } else {
                    true
                }
            }
        };
    }
}

#[derive(Component)]
pub struct NonOperationalIndicator;
pub fn update_operational_indicators(
    mut commands: Commands,
    mut buildings: Query<(Entity, &Operational), (With<Building>, Changed<Operational>)>,
    indicators: Query<Entity, With<NonOperationalIndicator>>,
    children: Query<&Children>,
) {
    for (building_entity, operational) in buildings.iter_mut() {
        let existing_indicator = children.get(building_entity)
            .ok()
            .and_then(|children| {
                children.iter().find(|&&child| indicators.contains(child))
            });

        match (operational.0, existing_indicator) {
            (false, None) => {
                let indicator = commands.spawn((
                    NonOperationalIndicator,
                    Text2d("!".to_string()),
                    TextFont {
                        font_size: 32.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(1.0, 0.0, 0.0)),
                    Transform::from_xyz(0.0, 0.0, 1.1),
                )).id();
                
                commands.entity(building_entity).add_child(indicator);
            }
            (true, Some(&indicator_entity)) => {
                commands.entity(indicator_entity).despawn();
            }
            _ => {}
        }
    }
}

pub fn calculate_network_connectivity(
    building_layers: &Query<(&BuildingType, &Position, &Layer), With<Building>>,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
) -> HashSet<(i32, i32)> {
    let mut connected_cells = HashSet::new();
    let mut queue = VecDeque::new();
    
    // Add hub positions as starting points
    for (multi_cell, _) in hub.iter() {
        let half_width = multi_cell.width / 2;
        let half_height = multi_cell.height / 2;
        
        for dy in -half_height..=half_height {
            for dx in -half_width..=half_width {
                let pos = (multi_cell.center_x + dx, multi_cell.center_y + dy);
                if connected_cells.insert(pos) {
                    queue.push_back(pos);
                }
            }
        }
    }
    
    // Flood fill to find all positions connected via connectors
    while let Some((x, y)) = queue.pop_front() {
        for (adj_x, adj_y) in [(x+1, y), (x-1, y), (x, y+1), (x, y-1)] {
            if connected_cells.contains(&(adj_x, adj_y)) {
                continue;
            }
            
            // Check if this adjacent position has a connector
            let has_connector = building_layers.iter().any(|(building_type, position, layer)| {
                layer.0 == BUILDING_LAYER && 
                *building_type == BuildingType::Connector &&
                position.x == adj_x && position.y == adj_y
            });
            
            if has_connector {
                connected_cells.insert((adj_x, adj_y));
                queue.push_back((adj_x, adj_y));
            }
        }
    }
    
    connected_cells
}

pub fn update_network_connectivity(
    mut network_connectivity: ResMut<NetworkConnectivity>,
    mut network_events: EventReader<NetworkChangedEvent>,
    building_layers: Query<(&BuildingType, &Position, &Layer), With<Building>>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
) {
    // Calculate on first run even without event, or when event received
    let should_update = network_events.len() > 0 || network_connectivity.connected_cells.is_empty();
    
    if !should_update {
        return;
    }
    
    network_events.clear();
    
    network_connectivity.connected_cells = calculate_network_connectivity(&building_layers, &hub);
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
    
    // Create visual connections between adjacent connected cells
    let positions: Vec<_> = network_connectivity.connected_cells.iter().cloned().collect();
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let (x1, y1) = positions[i];
            let (x2, y2) = positions[j];
            
            let dx = (x2 - x1).abs();
            let dy = (y2 - y1).abs();
            
            if (dx == 1 && dy == 0) || (dx == 0 && dy == 1) {
                spawn_connection_visual(&mut commands, (x1, y1), (x2, y2));
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