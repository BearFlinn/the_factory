use crate::{
    grid::Grid,
    workers::{WorkerBundle, WorkersSystemSet},
};
use bevy::prelude::*;

#[derive(Component)]
pub struct SpawnWorkerButton;

pub fn setup_spawn_worker_button(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                right: Val::Px(20.0),
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Button,
            SpawnWorkerButton,
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Spawn Worker"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn handle_spawn_worker_button(
    mut commands: Commands,
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<SpawnWorkerButton>),
    >,
    grid: Res<Grid>,
) {
    for (interaction, mut background_color) in &mut button_query {
        match *interaction {
            Interaction::Pressed => {
                let spawn_world_pos = grid.grid_to_world_coordinates(0, 0);
                commands.spawn(WorkerBundle::new(spawn_world_pos));
                println!("Manual worker spawned at world position: {spawn_world_pos:?}");
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
            }
            Interaction::None => {
                *background_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
        }
    }
}

pub struct SpawnWorkerButtonPlugin;

impl Plugin for SpawnWorkerButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_spawn_worker_button)
            .add_systems(
                Update,
                handle_spawn_worker_button.in_set(WorkersSystemSet::Lifecycle),
            );
    }
}
