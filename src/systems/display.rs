use crate::{
    grid::Grid,
    materials::{items::Inventory, ItemRegistry},
    structures::{building_config::BuildingRegistry, Building, PlaceBuildingValidationEvent},
    systems::Operational,
    ui::SelectedBuilding,
    workers::Worker,
};
use bevy::prelude::*;

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

#[allow(clippy::needless_pass_by_value, clippy::type_complexity)] // Bevy system parameters
pub fn update_inventory_display(
    mut commands: Commands,
    buildings_and_workers: Query<(Entity, &Inventory), Or<(With<Building>, With<Worker>)>>,
    mut inventory_displays: Query<&mut Text2d, With<InventoryDisplay>>,
    children: Query<&Children>,
    changed_inventories: Query<Entity, (Or<(With<Worker>, With<Building>)>, Changed<Inventory>)>,
    item_registry: Res<ItemRegistry>,
) {
    for (building_entity, inventory) in buildings_and_workers.iter() {
        let should_update = changed_inventories.contains(building_entity);

        let existing_display = children.get(building_entity).ok().and_then(|children| {
            children.iter().find_map(|&child| {
                if inventory_displays.contains(child) {
                    Some(child)
                } else {
                    None
                }
            })
        });

        // Format all items for display
        let display_text = if inventory.items.is_empty() {
            "Empty".to_string()
        } else {
            inventory
                .items
                .iter()
                .map(|(item_name, &quantity)| {
                    let name = item_registry
                        .get_definition(item_name)
                        .map_or("Unknown", |def| def.name.as_str());
                    format!("{name}: {quantity}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        if let Some(display_entity) = existing_display {
            if should_update {
                if let Ok(mut text) = inventory_displays.get_mut(display_entity) {
                    text.0 = display_text;
                }
            }
        } else {
            let display = commands
                .spawn((
                    InventoryDisplay,
                    Text2d::new(display_text),
                    TextFont {
                        font_size: 12.0, // Smaller font for multi-line display
                        ..Default::default()
                    },
                    TextColor(Color::srgb(0.2, 0.2, 0.2)),
                    Transform::from_xyz(0.0, 30.0, 1.1), // Higher offset for multi-line
                ))
                .id();

            commands.entity(building_entity).add_child(display);
        }
    }
}

#[allow(clippy::needless_pass_by_value, clippy::type_complexity)] // Bevy system parameters
pub fn update_operational_indicators(
    mut commands: Commands,
    mut buildings: Query<(Entity, &Operational), (With<Building>, Changed<Operational>)>,
    indicators: Query<Entity, With<NonOperationalIndicator>>,
    children: Query<&Children>,
) {
    for (building_entity, operational) in &mut buildings {
        let existing_indicator = children
            .get(building_entity)
            .ok()
            .and_then(|children| children.iter().find(|&&child| indicators.contains(child)));

        match (operational.get_status(), existing_indicator) {
            (false, None) => {
                let indicator = commands
                    .spawn((
                        NonOperationalIndicator,
                        Text2d("!".to_string()),
                        TextFont {
                            font_size: 32.0,
                            ..Default::default()
                        },
                        TextColor(Color::srgb(1.0, 0.0, 0.0)),
                        Transform::from_xyz(0.0, 0.0, 1.1),
                    ))
                    .id();

                commands.entity(building_entity).add_child(indicator);
            }
            (true, Some(&indicator_entity)) => {
                commands.entity(indicator_entity).despawn();
            }
            _ => {}
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Bevy system parameters
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
                if let Ok((_, mut transform, mut sprite, mut ghost)) = ghost_query.get_single_mut()
                {
                    // Update existing ghost position
                    let world_pos = grid.grid_to_world_coordinates(coords.grid_x, coords.grid_y);
                    transform.translation = Vec3::new(world_pos.x, world_pos.y, 0.5);

                    // Update sprite if building type changed
                    if ghost.building_name != *building_name {
                        sprite.color = Color::srgba(
                            def.appearance.color.0,
                            def.appearance.color.1,
                            def.appearance.color.2,
                            0.8,
                        );
                        sprite.custom_size = Some(def.appearance.size.into());
                        ghost.building_name.clone_from(building_name);
                    }
                } else {
                    // Create new ghost
                    let world_pos = grid.grid_to_world_coordinates(coords.grid_x, coords.grid_y);
                    commands.spawn((
                        PlacementGhost {
                            building_name: building_name.clone(),
                        },
                        Sprite::from_color(
                            Color::srgba(
                                def.appearance.color.0,
                                def.appearance.color.1,
                                def.appearance.color.2,
                                0.8,
                            ),
                            def.appearance.size.into(),
                        ),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.5),
                    ));
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

#[allow(clippy::needless_pass_by_value)] // Bevy system parameters
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

            let world_pos =
                grid.grid_to_world_coordinates(cursor_coords.grid_x, cursor_coords.grid_y);

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
#[allow(clippy::needless_pass_by_value)] // Bevy system parameters
pub fn cleanup_placement_errors(
    mut commands: Commands,
    time: Res<Time>,
    mut error_messages: Query<(Entity, &mut PlacementErrorMessage)>,
) {
    for (entity, mut error_msg) in &mut error_messages {
        error_msg.timer.tick(time.delta());
        if error_msg.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}
