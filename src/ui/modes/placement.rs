use bevy::prelude::*;

use crate::{
    grid::Grid,
    structures::{building_config::BuildingRegistry, PlaceBuildingValidationEvent},
    ui::SelectedBuilding,
};

#[derive(Component)]
pub struct PlacementGhost {
    pub building_name: String,
}

#[derive(Component)]
pub struct PlacementErrorMessage {
    pub timer: Timer,
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
            if let Some(def) = building_registry.get_definition(building_name) {
                if let Ok((_, mut transform, mut sprite, mut ghost)) = ghost_query.single_mut() {
                    let world_pos = grid.grid_to_world_coordinates(coords.grid_x, coords.grid_y);
                    transform.translation = Vec3::new(world_pos.x, world_pos.y, 0.5);

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
            for (entity, _, _, _) in ghost_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn display_placement_error(
    mut commands: Commands,
    mut validation_events: MessageReader<PlaceBuildingValidationEvent>,
    grid: Res<Grid>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    for event in validation_events.read() {
        if let Err(error) = &event.result {
            let Some(cursor_coords) = grid.get_cursor_grid_coordinates(&windows, &camera_q) else {
                continue;
            };

            let world_pos =
                grid.grid_to_world_coordinates(cursor_coords.grid_x, cursor_coords.grid_y);

            commands.spawn((
                PlacementErrorMessage {
                    timer: Timer::from_seconds(2.0, TimerMode::Once),
                },
                Text2d::new(error.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
                Transform::from_xyz(world_pos.x, world_pos.y + 40.0, 2.0),
            ));
        }
    }
}

pub fn cleanup_placement_errors(
    mut commands: Commands,
    time: Res<Time>,
    mut error_messages: Query<(Entity, &mut PlacementErrorMessage)>,
) {
    for (entity, mut error_msg) in &mut error_messages {
        error_msg.timer.tick(time.delta());
        if error_msg.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
