pub mod build_panel;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Checked;

use crate::{
    grid::Grid,
    ui::{
        icons::{GameIcon, IconAtlas},
        style::{
            ButtonStyle, ACTION_BAR_BG, ACTION_BAR_WIDTH, ACTION_BUTTON_SIZE, PANEL_BORDER,
            TOP_BAR_HEIGHT,
        },
        UISystemSet, UiMode,
    },
    workers::{WorkerBundle, WorkersSystemSet},
};

use build_panel::{despawn_build_panel, spawn_build_panel, BuildPanel};

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum ActivePanel {
    #[default]
    None,
    Build,
    Workflows,
}

#[derive(Component)]
pub struct ActionBar;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum ActionBarButton {
    Build,
    Workflows,
    SpawnWorker,
    FactoryInfo,
}

fn setup_action_bar(mut commands: Commands, icon_atlas: Res<IconAtlas>) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(TOP_BAR_HEIGHT),
            bottom: Val::Px(0.0),
            width: Val::Px(ACTION_BAR_WIDTH),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|outer| {
            outer
                .spawn((
                    Node {
                        height: Val::Percent(80.0),
                        width: Val::Px(ACTION_BAR_WIDTH),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::vertical(Val::Px(4.0)),
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(ACTION_BAR_BG),
                    ActionBar,
                ))
                .with_children(|parent| {
                    spawn_action_button(
                        parent,
                        &icon_atlas,
                        GameIcon::Build,
                        ActionBarButton::Build,
                    );
                    spawn_action_button(
                        parent,
                        &icon_atlas,
                        GameIcon::Workflows,
                        ActionBarButton::Workflows,
                    );
                    spawn_action_button(
                        parent,
                        &icon_atlas,
                        GameIcon::SpawnWorker,
                        ActionBarButton::SpawnWorker,
                    );
                    spawn_action_button(
                        parent,
                        &icon_atlas,
                        GameIcon::FactoryInfo,
                        ActionBarButton::FactoryInfo,
                    );
                });
        });
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands,
    icon_atlas: &IconAtlas,
    icon: GameIcon,
    action: ActionBarButton,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(ACTION_BUTTON_SIZE),
                height: Val::Px(ACTION_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(crate::ui::style::ACTION_BUTTON_BG),
            BorderColor::all(PANEL_BORDER),
            ButtonStyle::action_bar(),
            Hovered::default(),
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                ImageNode {
                    image: icon_atlas.image.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: icon_atlas.layout.clone(),
                        index: icon as usize,
                    }),
                    ..default()
                },
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    ..default()
                },
            ));
        });
}

fn handle_action_bar_clicks(
    mut commands: Commands,
    button_query: Query<
        (Entity, &ActionBarButton, &Interaction),
        (Changed<Interaction>, With<ActionBarButton>),
    >,
    mut active_panel: ResMut<ActivePanel>,
    grid: Res<Grid>,
) {
    for (_entity, action, interaction) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            ActionBarButton::Build => {
                if *active_panel == ActivePanel::Build {
                    *active_panel = ActivePanel::None;
                } else {
                    *active_panel = ActivePanel::Build;
                }
            }
            ActionBarButton::Workflows => {
                if *active_panel == ActivePanel::Workflows {
                    *active_panel = ActivePanel::None;
                } else {
                    *active_panel = ActivePanel::Workflows;
                }
            }
            ActionBarButton::SpawnWorker => {
                let spawn_world_pos = grid.grid_to_world_coordinates(0, 0);
                commands.spawn(WorkerBundle::new(spawn_world_pos));
                info!("manual worker spawned at world position: {spawn_world_pos:?}");
            }
            ActionBarButton::FactoryInfo => {
                info!("factory info placeholder");
            }
        }
    }
}

fn handle_action_bar_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active_panel: ResMut<ActivePanel>,
    current_mode: Res<State<UiMode>>,
) {
    if *current_mode.get() == UiMode::WorkflowCreate {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        if *active_panel == ActivePanel::Build {
            *active_panel = ActivePanel::None;
        } else {
            *active_panel = ActivePanel::Build;
        }
    }

    if keyboard.just_pressed(KeyCode::Tab) {
        if *active_panel == ActivePanel::Build {
            *active_panel = ActivePanel::None;
        } else {
            *active_panel = ActivePanel::Build;
        }
    }
}

fn manage_panel_lifecycle(
    mut commands: Commands,
    active_panel: Res<ActivePanel>,
    build_panels: Query<Entity, With<BuildPanel>>,
    workflow_panels: Query<Entity, With<crate::ui::panels::workflow_list::WorkflowPanel>>,
    registry: Res<crate::structures::BuildingRegistry>,
    icon_atlas: Res<IconAtlas>,
) {
    if !active_panel.is_changed() {
        return;
    }

    for entity in &build_panels {
        despawn_build_panel(&mut commands, entity);
    }
    for entity in &workflow_panels {
        commands.entity(entity).despawn();
    }

    match *active_panel {
        ActivePanel::Build => {
            spawn_build_panel(&mut commands, &registry, &icon_atlas);
        }
        ActivePanel::Workflows => {
            crate::ui::panels::workflow_list::spawn_workflow_panel(&mut commands);
        }
        ActivePanel::None => {}
    }
}

fn sync_action_bar_checked(
    mut commands: Commands,
    active_panel: Res<ActivePanel>,
    buttons: Query<(Entity, &ActionBarButton)>,
) {
    if !active_panel.is_changed() {
        return;
    }

    for (entity, action) in &buttons {
        let should_be_checked = match action {
            ActionBarButton::Build => *active_panel == ActivePanel::Build,
            ActionBarButton::Workflows => *active_panel == ActivePanel::Workflows,
            _ => false,
        };

        if should_be_checked {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).remove::<Checked>();
        }
    }
}

fn clear_selection_on_panel_close(
    active_panel: Res<ActivePanel>,
    mut selected_building: ResMut<crate::ui::SelectedBuilding>,
) {
    if !active_panel.is_changed() {
        return;
    }
    if *active_panel != ActivePanel::Build && selected_building.building_name.is_some() {
        selected_building.building_name = None;
    }
}

pub struct ActionBarPlugin;

impl Plugin for ActionBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePanel>()
            .add_systems(PostStartup, setup_action_bar)
            .add_systems(
                Update,
                (
                    (handle_action_bar_hotkeys,).in_set(UISystemSet::InputDetection),
                    (
                        handle_action_bar_clicks.in_set(WorkersSystemSet::Lifecycle),
                        (manage_panel_lifecycle, clear_selection_on_panel_close)
                            .in_set(UISystemSet::EntityManagement),
                        sync_action_bar_checked.in_set(UISystemSet::VisualUpdates),
                    ),
                ),
            );
    }
}
