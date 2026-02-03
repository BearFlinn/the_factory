use std::collections::{HashMap, HashSet};

use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::{
    ui::{
        popups::building_menu::BuildingClickEvent,
        style::{
            ButtonStyle, CANCEL_BG, CONFIRM_BG, DIM_TEXT, HEADER_COLOR, PANEL_BG, PANEL_BORDER,
            TEXT_COLOR,
        },
        UISystemSet,
    },
    workers::workflows::components::WorkflowStep,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub enum CreationPhase {
    #[default]
    SelectBuildings,
    BuilderModal,
}

#[derive(Resource, Default)]
pub struct WorkflowCreationState {
    pub name: String,
    pub building_set: HashSet<Entity>,
    pub steps: Vec<WorkflowStep>,
    pub desired_worker_count: u32,
    pub phase: CreationPhase,
}

#[derive(Resource, Default)]
pub struct WorkflowCreationCounter {
    count: u32,
}

#[derive(Component)]
pub struct WorkflowCreationPanel;

#[derive(Component)]
pub struct WorkflowCancelButton;

#[derive(Component)]
pub struct BuildingPoolList;

#[derive(Component)]
pub struct BuildWorkflowButton;

fn toggle_workflow_creation_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<WorkflowCreationState>,
    mut counter: ResMut<WorkflowCreationCounter>,
    mut commands: Commands,
    existing_panels: Query<Entity, With<WorkflowCreationPanel>>,
    mut next_mode: ResMut<NextState<crate::ui::UiMode>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyN) {
        return;
    }

    counter.count += 1;
    state.name = format!("Workflow {}", counter.count);
    state.steps.clear();
    state.desired_worker_count = 1;
    state.building_set.clear();
    state.phase = CreationPhase::SelectBuildings;

    for entity in &existing_panels {
        commands.entity(entity).despawn();
    }

    spawn_creation_panel(&mut commands, &state);
    next_mode.set(crate::ui::UiMode::WorkflowCreate);
}

pub(crate) fn spawn_creation_panel(commands: &mut Commands, state: &WorkflowCreationState) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                width: Val::Px(500.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                margin: UiRect::horizontal(Val::Auto),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(PANEL_BORDER),
            WorkflowCreationPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Creating: {} - Select Buildings", state.name)),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(HEADER_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Click buildings on the grid to add/remove them from the pool."),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
            ));

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        min_height: Val::Px(30.0),
                        ..default()
                    },
                    BuildingPoolList,
                ))
                .with_children(|pool| {
                    pool.spawn((
                        Text::new("No buildings selected."),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(DIM_TEXT),
                    ));
                });

            spawn_phase1_buttons(parent);
        });
}

fn spawn_phase1_buttons(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(34.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Button,
                Node {
                    width: Val::Px(90.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(CANCEL_BG),
                BorderColor::all(Color::srgb(0.5, 0.3, 0.3)),
                ButtonStyle::cancel(),
                Hovered::default(),
                WorkflowCancelButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("Cancel"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });

            row.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(CONFIRM_BG),
                BorderColor::all(Color::srgb(0.3, 0.5, 0.3)),
                ButtonStyle::confirm(),
                Hovered::default(),
                BuildWorkflowButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("Build Workflow ->"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
        });
}

fn handle_building_pool_clicks(
    mut state: ResMut<WorkflowCreationState>,
    mut click_events: MessageReader<BuildingClickEvent>,
    mut commands: Commands,
    pool_lists: Query<(Entity, &Children), With<BuildingPoolList>>,
    names: Query<&Name>,
) {
    if state.phase != CreationPhase::SelectBuildings {
        return;
    }

    for click in click_events.read() {
        let entity = click.building_entity;
        if state.building_set.contains(&entity) {
            state.building_set.remove(&entity);
        } else {
            state.building_set.insert(entity);
        }

        rebuild_building_pool_list(&mut commands, &pool_lists, &state.building_set, &names);
    }
}

fn rebuild_building_pool_list(
    commands: &mut Commands,
    pool_lists: &Query<(Entity, &Children), With<BuildingPoolList>>,
    building_set: &HashSet<Entity>,
    names: &Query<&Name>,
) {
    for (list_entity, children) in pool_lists {
        for &child in children {
            commands.entity(child).despawn();
        }

        commands.entity(list_entity).with_children(|parent| {
            if building_set.is_empty() {
                parent.spawn((
                    Text::new("No buildings selected."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
                return;
            }

            let mut type_counts: HashMap<String, u32> = HashMap::new();
            for &entity in building_set {
                let name = names
                    .get(entity)
                    .map_or_else(|_| "Unknown".to_string(), |n| n.as_str().to_string());
                *type_counts.entry(name).or_default() += 1;
            }

            let mut types: Vec<_> = type_counts.into_iter().collect();
            types.sort_by(|a, b| a.0.cmp(&b.0));

            let summary = types
                .iter()
                .map(|(name, count)| {
                    if *count > 1 {
                        format!("{count}x {name}")
                    } else {
                        name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            parent.spawn((
                Text::new(summary),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
    }
}

fn handle_phase1_controls(
    mut state: ResMut<WorkflowCreationState>,
    cancel_buttons: Query<&Interaction, (Changed<Interaction>, With<WorkflowCancelButton>)>,
    build_buttons: Query<&Interaction, (Changed<Interaction>, With<BuildWorkflowButton>)>,
    mut commands: Commands,
    panels: Query<Entity, With<WorkflowCreationPanel>>,
    mut next_mode: ResMut<NextState<crate::ui::UiMode>>,
) {
    if state.phase != CreationPhase::SelectBuildings {
        return;
    }

    for interaction in &cancel_buttons {
        if *interaction == Interaction::Pressed {
            next_mode.set(crate::ui::UiMode::Observe);
            return;
        }
    }

    for interaction in &build_buttons {
        if *interaction == Interaction::Pressed && state.building_set.len() >= 2 {
            state.phase = CreationPhase::BuilderModal;
            for entity in &panels {
                commands.entity(entity).despawn();
            }
            return;
        }
    }
}

fn respawn_panel_on_phase_back(
    state: Res<WorkflowCreationState>,
    mut commands: Commands,
    existing_panels: Query<Entity, With<WorkflowCreationPanel>>,
) {
    if !state.is_changed() {
        return;
    }
    if state.phase != CreationPhase::SelectBuildings {
        return;
    }
    if !existing_panels.is_empty() {
        return;
    }
    spawn_creation_panel(&mut commands, &state);
}

pub struct WorkflowCreationPlugin;

impl Plugin for WorkflowCreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorkflowCreationState>()
            .init_resource::<WorkflowCreationCounter>()
            .add_systems(
                Update,
                (
                    toggle_workflow_creation_mode
                        .in_set(UISystemSet::InputDetection)
                        .run_if(in_state(crate::ui::UiMode::Observe)),
                    (
                        handle_phase1_controls,
                        handle_building_pool_clicks,
                        respawn_panel_on_phase_back,
                    )
                        .in_set(UISystemSet::EntityManagement)
                        .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
                ),
            );
    }
}
