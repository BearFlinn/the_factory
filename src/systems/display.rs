use bevy::prelude::*;
use crate::{
    structures::{Building, construction::BuildingRegistry, PlaceBuildingValidationEvent}, 
    workers::Worker, 
    items::Inventory, 
    systems::Operational,
    ui::SelectedBuilding,
    grid::Grid,
};

#[derive(Component)]
pub struct InventoryDisplay;

#[derive(Component)]
pub struct NonOperationalIndicator;

#[derive(Component)]
pub struct PlacementGhost {
    pub building_name: String,
}

#[derive(Component)]
pub struct PlacementErrorMessage {
    pub timer: Timer,
}

pub fn update_inventory_display(
    mut commands: Commands,
    buildings_and_workers: Query<(Entity, &Inventory), Or<(With<Building>, With<Worker>)>>,
    mut inventory_displays: Query<&mut Text2d, With<InventoryDisplay>>,
    children: Query<&Children>,
    changed_inventories: Query<Entity, (Or<(With<Worker>, With<Building>)>, Changed<Inventory>)>,
) {
    for (building_entity, inventory) in buildings_and_workers.iter() {
        // Check if this building's inventory changed, or if we need to create initial display
        let should_update = changed_inventories.contains(building_entity);
        
        let existing_display = children.get(building_entity)
            .ok()
            .and_then(|children| {
                children.iter().find_map(|&child| {
                    if inventory_displays.contains(child) {
                        Some(child)
                    } else {
                        None
                    }
                })
            });

        match existing_display {
            Some(display_entity) => {
                // Update existing display if inventory changed
                if should_update {
                    if let Ok(mut text) = inventory_displays.get_mut(display_entity) {
                        text.0 = format!("{}", inventory.get_item_quantity(0));
                    }
                }
            }
            None => {
                // Create new display
                let display = commands.spawn((
                    InventoryDisplay,
                    Text2d::new(format!("{}", inventory.get_item_quantity(0))),
                    TextFont {
                        font_size: 16.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    Transform::from_xyz(0.0, 0.0, 1.1), // Position above building
                )).id();

                commands.entity(building_entity).add_child(display);
            }
        }
    }
}

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

pub fn update_placement_ghost(
    mut commands: Commands,
    selected_building: Res<SelectedBuilding>,
    building_registry: Res<BuildingRegistry>,
    grid: Res<Grid>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut ghost_query: Query<(Entity, &mut Transform, &mut Sprite, &mut PlacementGhost)>,
) {
    let cursor_coords = grid.get_cursor_grid_coordinates(&windows, &camera_q);
    
    match (&selected_building.building_name, cursor_coords) {
        (Some(building_name), Some(coords)) => {
            // Building selected and cursor on valid grid - show/update ghost
            if let Some(def) = building_registry.get_definition(building_name) {
                match ghost_query.get_single_mut() {
                    Ok((_, mut transform, mut sprite, mut ghost)) => {
                        // Update existing ghost position
                        let world_pos = grid.grid_to_world_coordinates(coords.grid_x, coords.grid_y);
                        transform.translation = Vec3::new(world_pos.x, world_pos.y, 0.5);
                        
                        // Update sprite if building type changed
                        if ghost.building_name != *building_name {
                            sprite.color = def.color.with_alpha(0.8);
                            sprite.custom_size = Some(def.size);
                            ghost.building_name = building_name.clone();
                        }
                    }
                    Err(_) => {
                        // Create new ghost
                        let world_pos = grid.grid_to_world_coordinates(coords.grid_x, coords.grid_y);
                        commands.spawn((
                            PlacementGhost {
                                building_name: building_name.clone(),
                            },
                            Sprite::from_color(def.color.with_alpha(0.5), def.size),
                            Transform::from_xyz(world_pos.x, world_pos.y, 0.5),
                        ));
                    }
                }
            }
        }
        _ => {
            // No building selected or cursor not on grid - remove ghost
            for (entity, _, _, _) in ghost_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn display_placement_error(
    mut commands: Commands,
    mut validation_events: EventReader<PlaceBuildingValidationEvent>,
    grid: Res<Grid>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    for event in validation_events.read() {
        // Only handle error cases
        if let Err(error) = &event.result {
            let Some(cursor_coords) = grid.get_cursor_grid_coordinates(&windows, &camera_q) else {
                continue;
            };
            
            let world_pos = grid.grid_to_world_coordinates(cursor_coords.grid_x, cursor_coords.grid_y);
            
            // Spawn floating error message
            commands.spawn((
                PlacementErrorMessage {
                    timer: Timer::from_seconds(2.0, TimerMode::Once),
                },
                Text2d::new(error.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.3, 0.3)), // Red error text
                Transform::from_xyz(world_pos.x, world_pos.y + 40.0, 2.0), // Offset above cursor
            ));
        }
    }
}

// Add to display.rs - cleanup expired error messages
pub fn cleanup_placement_errors(
    mut commands: Commands,
    time: Res<Time>,
    mut error_messages: Query<(Entity, &mut PlacementErrorMessage)>,
) {
    for (entity, mut error_msg) in error_messages.iter_mut() {
        error_msg.timer.tick(time.delta());
        if error_msg.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}