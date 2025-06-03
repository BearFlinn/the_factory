use bevy::prelude::*;

use crate::grid::{CellChildren, Layer, Position};

use super::{is_connected_to_network, Building, BuildingType, Hub, MultiCellBuilding, Operational, PowerConsumer, PowerGenerator, Producer, BUILDING_LAYER};

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

pub fn update_operational_status(
    mut buildings: Query<(&BuildingType, &Position, &mut Operational, Option<&PowerConsumer>), With<Building>>,
    grid_cells: Query<(Entity, &Position, &CellChildren)>,
    building_layers: Query<(&BuildingType, &Layer)>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
    power_grid: Res<PowerGrid>,
) {
    let has_power = power_grid.available >= 0;
    
    for (building_type, pos, mut operational, power_consumer) in buildings.iter_mut() {
        // First check network connectivity
        if !is_connected_to_network(&grid_cells, &building_layers, &hub, pos.x, pos.y) {
            operational.0 = false;
            continue; 
        }
        
        // Then check power requirements
        operational.0 = match building_type {
            BuildingType::Generator => true, // Generators don't need power
            _ => {
                // Other buildings need power if they consume it
                if power_consumer.is_some() {
                    has_power
                } else {
                    true // Buildings without power consumption are always operational if connected
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
        // Find existing indicator if any
        let existing_indicator = children.get(building_entity)
            .ok()
            .and_then(|children| {
                children.iter().find(|&&child| indicators.contains(child))
            });

        match (operational.0, existing_indicator) {
            // Building became non-operational and has no indicator - add one
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
            
            // Building became operational and has indicator - remove it
            (true, Some(&indicator_entity)) => {
                commands.entity(indicator_entity).despawn();
            }
            
            // No change needed
            _ => {}
        }
    }
}

pub fn update_network_connections(
    mut commands: Commands,
    mut network_events: EventReader<NetworkChangedEvent>,
    building_layers: Query<(&BuildingType, &Layer, &Position), With<super::Building>>,
    hub: Query<(&MultiCellBuilding, &Hub)>,
    existing_connections: Query<Entity, (With<NetworkConnection>, Changed<NetworkConnection>)>,
) {
    // Only update when network changes
    if network_events.is_empty() {
        return;
    }
    network_events.clear();

    // Remove existing connection visuals
    for entity in existing_connections.iter() {
        commands.entity(entity).despawn();
    }

    // Get all network building positions
    let mut network_positions = Vec::new();
    
    // Add hub positions
    for (multi_cell, _) in hub.iter() {
        let half_width = multi_cell.width / 2;
        let half_height = multi_cell.height / 2;
        
        for dy in -half_height..=half_height {
            for dx in -half_width..=half_width {
                network_positions.push((
                    multi_cell.center_x + dx,
                    multi_cell.center_y + dy,
                    BuildingType::Generator
                ));
            }
        }
    }
    
    // Add connector positions
    for (building_type, layer, pos) in building_layers.iter() {
        if layer.0 == BUILDING_LAYER && *building_type == BuildingType::Connector {
            network_positions.push((pos.x, pos.y, *building_type));
        }
    }

    // Create connections between adjacent network buildings
    for i in 0..network_positions.len() {
        for j in (i + 1)..network_positions.len() {
            let (x1, y1, _) = network_positions[i];
            let (x2, y2, _) = network_positions[j];
            
            // Check if positions are adjacent
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