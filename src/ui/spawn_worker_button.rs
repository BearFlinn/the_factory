use bevy::prelude::*;
use crate::{
    grid::Grid, ui::{DynamicStyles, InteractiveUI, Selectable, SelectionBehavior}, workers::{WorkerBundle, WorkersSystemSet}
};

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
            Selectable::new().with_behavior(SelectionBehavior::Toggle),
            InteractiveUI::new()
                .default(DynamicStyles::new().with_background(Color::srgb(0.2, 0.2, 0.2)))
                .on_hover(DynamicStyles::new().with_background(Color::srgb(0.3, 0.3, 0.3)))
                .on_click(DynamicStyles::new().with_background(Color::srgb(0.1, 0.1, 0.1))),
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
    button_query: Query<&Selectable, (With<SpawnWorkerButton>, Changed<Selectable>)>,
    grid: Res<Grid>,
) {
    for selectable in &button_query {
        if selectable.is_selected {
            // Spawn worker at origin (0, 0) grid position
            let spawn_world_pos = grid.grid_to_world_coordinates(0, 0);
            
            commands.spawn(WorkerBundle::new(
                spawn_world_pos,
            ));
            
            println!("Manual worker spawned at world position: {:?}", spawn_world_pos);
        }
    }
}

pub struct SpawnWorkerButtonPlugin;

impl Plugin for SpawnWorkerButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_spawn_worker_button)
           .add_systems(Update, handle_spawn_worker_button.in_set(WorkersSystemSet::Lifecycle));
    }
}