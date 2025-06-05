use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};

use crate::ui::SelectedBuilding;

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
            base_speed: 400.0,        // Base movement speed in pixels/second
            acceleration: 8.0,        // How quickly we reach target speed
            deceleration: 12.0,       // How quickly we stop when no input
            min_zoom: 0.3,           // Maximum zoom in
            max_zoom: 3.0,           // Maximum zoom out
        }
    }
}

#[derive(Resource, Default)]
pub struct CameraInput {
    pub is_dragging: bool,
    pub last_mouse_position: Option<Vec2>,
}

#[derive(Resource, Default)]
pub struct CameraControl {
    pub panning_enabled: bool,
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        GameCamera::default(),
    ));
    commands.insert_resource(CameraInput::default());
    commands.insert_resource(CameraControl { panning_enabled: true });
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

    // Calculate target velocity based on input
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

    // Normalize diagonal movement
    if target_velocity.length() > 0.0 {
        target_velocity = target_velocity.normalize();
    }

    // Scale movement speed by zoom level (move faster when zoomed out)
    let zoom_scale = projection.scale;
    target_velocity *= game_camera.base_speed * zoom_scale;

    // Apply acceleration/deceleration
    let delta_time = time.delta_secs();
    
    if target_velocity.length() > 0.0 {
        // Accelerate toward target velocity
        game_camera.velocity = game_camera.velocity.lerp(
            target_velocity, 
            game_camera.acceleration * delta_time
        );
    } else {
        // Decelerate to zero
        game_camera.velocity = game_camera.velocity.lerp(
            Vec2::ZERO, 
            game_camera.deceleration * delta_time
        );
    }

    // Apply velocity to camera position
    camera_transform.translation += game_camera.velocity.extend(0.0) * delta_time;
}

pub fn handle_camera_mouse_drag(
    mut camera_input: ResMut<CameraInput>,
    camera_control: Res<CameraControl>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    windows: Query<&Window>,
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
    projection_query: Query<&OrthographicProjection, With<Camera2d>>,
) {
    // Early exit if panning is disabled
    if !camera_control.panning_enabled {
        camera_input.is_dragging = false;
        camera_input.last_mouse_position = None;
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    
    let Ok(mut camera_transform) = camera_transform_query.get_single_mut() else {
        return;
    };

    let Ok(projection) = projection_query.get_single() else {
        return;
    };

    // Handle mouse drag start/end
    if mouse_button.just_pressed(MouseButton::Left) {
        camera_input.is_dragging = true;
        camera_input.last_mouse_position = window.cursor_position();
    }
    
    if mouse_button.just_released(MouseButton::Left) {
        camera_input.is_dragging = false;
        camera_input.last_mouse_position = None;
    }

    // Handle dragging
    if camera_input.is_dragging {
        for motion in mouse_motion.read() {
            // Convert mouse delta to world space
            let screen_to_world_scale = projection.scale;
            
            // Invert the motion (drag left should move camera right to make world appear to move left)
            let world_delta = Vec2::new(-motion.delta.x, motion.delta.y) * screen_to_world_scale;
            
            camera_transform.translation += world_delta.extend(0.0);
        }
    }
}

pub fn handle_camera_zoom(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
    mut projection_query: Query<&mut OrthographicProjection, With<Camera2d>>,
    game_camera_query: Query<&GameCamera, With<Camera2d>>,
) {
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
        // Get cursor position in world coordinates before zoom
        let cursor_world_pos = window.cursor_position()
            .and_then(|cursor_pos| camera.viewport_to_world(camera_global_transform, cursor_pos).ok())
            .map(|ray| ray.origin.truncate());

        if let Some(cursor_world_before) = cursor_world_pos {
            // Calculate zoom factor
            let zoom_factor = 1.0 + scroll.y * -0.1; // Negative because scroll up should zoom in
            let new_scale = (projection.scale * zoom_factor).clamp(game_camera.min_zoom, game_camera.max_zoom);
            
            // Only apply zoom if it's within bounds
            if new_scale != projection.scale {
                projection.scale = new_scale;

                // Get cursor position in world coordinates after zoom
                let cursor_world_after = window.cursor_position()
                    .and_then(|cursor_pos| camera.viewport_to_world(camera_global_transform, cursor_pos).ok())
                    .map(|ray| ray.origin.truncate());

                if let Some(cursor_world_after) = cursor_world_after {
                    // Adjust camera position to keep cursor at same world position
                    let world_delta = cursor_world_before - cursor_world_after;
                    camera_transform.translation += world_delta.extend(0.0);
                }
            }
        }
    }
}

pub fn update_camera_control(
    mut camera_control: ResMut<CameraControl>,
    selected_building: Res<SelectedBuilding>,
    ui_interactions: Query<&Interaction, With<Button>>,
) {
    // Disable panning if any UI element is being interacted with
    let ui_active = ui_interactions.iter().any(|interaction| {
        matches!(interaction, Interaction::Pressed | Interaction::Hovered)
    });
    
    // Disable panning if a building is selected for placement
    let building_selected = selected_building.building_id.is_some();
    
    camera_control.panning_enabled = !ui_active && !building_selected;
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_camera)
            .add_systems(Update, (
                update_camera_control,
                handle_camera_keyboard_input,
                handle_camera_mouse_drag,
                handle_camera_zoom,
            ));
    }
}