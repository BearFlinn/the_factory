use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

#[derive(Component)]
pub struct GameCamera {
    pub velocity: Vec2,
    pub base_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            base_speed: 650.0,
            acceleration: 8.0,
            deceleration: 12.0,
            min_zoom: 0.3,
            max_zoom: 3.0,
        }
    }
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, GameCamera::default()));
}

pub fn handle_camera_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &mut GameCamera), With<Camera2d>>,
    projection_query: Query<&OrthographicProjection, With<Camera2d>>,
) {
    let Ok((mut camera_transform, mut game_camera)) = camera_query.get_single_mut() else {
        return;
    };

    let Ok(projection) = projection_query.get_single() else {
        return;
    };

    let mut target_velocity = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        target_velocity.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        target_velocity.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        target_velocity.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        target_velocity.x += 1.0;
    }

    if target_velocity.length() > 0.0 {
        target_velocity = target_velocity.normalize();
    }

    let zoom_scale = projection.scale;
    target_velocity *= game_camera.base_speed * zoom_scale;

    let delta_time = time.delta_secs();

    if target_velocity.length() > 0.0 {
        game_camera.velocity = game_camera
            .velocity
            .lerp(target_velocity, game_camera.acceleration * delta_time);
    } else {
        game_camera.velocity = game_camera
            .velocity
            .lerp(Vec2::ZERO, game_camera.deceleration * delta_time);
    }

    camera_transform.translation += game_camera.velocity.extend(0.0) * delta_time;
}

pub fn handle_camera_zoom(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
    mut projection_query: Query<&mut OrthographicProjection, With<Camera2d>>,
    game_camera_query: Query<&GameCamera, With<Camera2d>>,
    ui_interactions: Query<&Interaction>,
) {
    let over_ui = ui_interactions
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered));

    let Ok(window) = windows.get_single() else {
        return;
    };

    let Ok((camera, camera_global_transform)) = camera_q.get_single() else {
        return;
    };

    let Ok(mut camera_transform) = camera_transform_query.get_single_mut() else {
        return;
    };

    let Ok(mut projection) = projection_query.get_single_mut() else {
        return;
    };

    let Ok(game_camera) = game_camera_query.get_single() else {
        return;
    };

    for scroll in mouse_wheel.read() {
        if over_ui {
            continue;
        }
        let cursor_world_pos = window
            .cursor_position()
            .and_then(|cursor_pos| {
                camera
                    .viewport_to_world(camera_global_transform, cursor_pos)
                    .ok()
            })
            .map(|ray| ray.origin.truncate());

        if let Some(cursor_world_before) = cursor_world_pos {
            let zoom_factor = 1.0 + scroll.y * -0.1;
            let new_scale =
                (projection.scale * zoom_factor).clamp(game_camera.min_zoom, game_camera.max_zoom);

            if (new_scale - projection.scale).abs() > f32::EPSILON {
                projection.scale = new_scale;

                let cursor_world_after = window
                    .cursor_position()
                    .and_then(|cursor_pos| {
                        camera
                            .viewport_to_world(camera_global_transform, cursor_pos)
                            .ok()
                    })
                    .map(|ray| ray.origin.truncate());

                if let Some(cursor_world_after) = cursor_world_after {
                    let world_delta = cursor_world_before - cursor_world_after;
                    camera_transform.translation += world_delta.extend(0.0);
                }
            }
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (handle_camera_keyboard_input, handle_camera_zoom));
    }
}
