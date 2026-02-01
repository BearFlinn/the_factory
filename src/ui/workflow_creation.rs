use bevy::prelude::*;

use crate::{
    ui::{BuildingClickEvent, UISystemSet},
    workers::workflows::components::{CreateWorkflowEvent, WorkflowAction, WorkflowStep},
};

#[derive(Resource, Default)]
pub struct WorkflowCreationState {
    pub active: bool,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub desired_worker_count: u32,
    pub pending_building: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct WorkflowCreationCounter {
    count: u32,
}

#[derive(Component)]
pub struct WorkflowCreationPanel;

#[derive(Component)]
pub struct WorkflowActionPopup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowActionType {
    Pickup,
    Dropoff,
}

#[derive(Component)]
pub struct WorkflowActionButton {
    pub action_type: WorkflowActionType,
}

#[derive(Component)]
pub struct WorkflowStepDisplay;

#[derive(Component)]
pub struct WorkflowWorkerCountLabel;

#[derive(Component)]
pub struct WorkflowConfirmButton;

#[derive(Component)]
pub struct WorkflowCancelButton;

#[derive(Component)]
pub struct WorkflowWorkerIncrementButton;

#[derive(Component)]
pub struct WorkflowWorkerDecrementButton;

#[derive(Component)]
pub struct WorkflowStepRemoveButton {
    step_index: usize,
}

#[derive(Component)]
pub struct WorkflowStepList;

const PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
const PANEL_BORDER: Color = Color::srgb(0.3, 0.4, 0.6);
const HEADER_COLOR: Color = Color::srgb(0.85, 0.85, 0.95);
const TEXT_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const DIM_TEXT: Color = Color::srgb(0.5, 0.5, 0.5);
const BUTTON_BG: Color = Color::srgb(0.2, 0.2, 0.3);
const BUTTON_HOVER: Color = Color::srgb(0.3, 0.3, 0.45);
const CONFIRM_BG: Color = Color::srgb(0.15, 0.35, 0.15);
const CONFIRM_HOVER: Color = Color::srgb(0.2, 0.5, 0.2);
const CANCEL_BG: Color = Color::srgb(0.35, 0.15, 0.15);
const CANCEL_HOVER: Color = Color::srgb(0.5, 0.2, 0.2);
const POPUP_BG: Color = Color::srgba(0.1, 0.1, 0.15, 0.95);

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
    state.pending_building = None;

    for entity in &existing_panels {
        commands.entity(entity).despawn();
    }

    spawn_creation_panel(&mut commands, &state);
    next_mode.set(crate::ui::UiMode::WorkflowCreate);
}

fn spawn_creation_panel(commands: &mut Commands, state: &WorkflowCreationState) {
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
                Text::new(format!("Creating Workflow: {}", state.name)),
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

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        min_height: Val::Px(30.0),
                        ..default()
                    },
                    WorkflowStepList,
                ))
                .with_children(|step_list| {
                    step_list.spawn((
                        Text::new("No steps added. Click a building to add steps."),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(DIM_TEXT),
                    ));
                });

            spawn_worker_count_row(parent, state.desired_worker_count);
            spawn_bottom_buttons(parent);
        });
}

fn spawn_worker_count_row(parent: &mut ChildSpawnerCommands, count: u32) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(30.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new("Workers:"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));

            row.spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_BG),
                WorkflowWorkerDecrementButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("-"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });

            row.spawn((
                Text::new(format!("{count}")),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
                WorkflowWorkerCountLabel,
            ));

            row.spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BUTTON_BG),
                WorkflowWorkerIncrementButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("+"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
        });
}

fn spawn_bottom_buttons(parent: &mut ChildSpawnerCommands) {
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
                    width: Val::Px(90.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(CONFIRM_BG),
                BorderColor::all(Color::srgb(0.3, 0.5, 0.3)),
                WorkflowConfirmButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("Confirm"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            });
        });
}

fn handle_building_click_in_creation_mode(
    mut state: ResMut<WorkflowCreationState>,
    mut click_events: MessageReader<BuildingClickEvent>,
    mut commands: Commands,
    existing_popups: Query<Entity, With<WorkflowActionPopup>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
) {
    for click in click_events.read() {
        for popup in &existing_popups {
            commands.entity(popup).despawn();
        }

        state.pending_building = Some(click.building_entity);

        let Ok((camera, camera_transform)) = camera_q.single() else {
            continue;
        };
        let Ok(window) = windows.single() else {
            continue;
        };
        let Some(screen_pos) = camera
            .world_to_viewport(camera_transform, click.world_position.extend(0.0))
            .ok()
        else {
            continue;
        };

        let popup_x = (screen_pos.x - 60.0).clamp(10.0, window.width() - 140.0);
        let popup_y = (screen_pos.y - 70.0).clamp(10.0, window.height() - 80.0);

        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(popup_x),
                    top: Val::Px(popup_y),
                    width: Val::Px(130.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(POPUP_BG),
                BorderColor::all(PANEL_BORDER),
                WorkflowActionPopup,
            ))
            .with_children(|popup| {
                popup.spawn((
                    Text::new("Select action:"),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));

                spawn_action_button(popup, "Pickup", WorkflowActionType::Pickup);
                spawn_action_button(popup, "Dropoff", WorkflowActionType::Dropoff);
            });
    }
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action_type: WorkflowActionType,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(26.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_BG),
            WorkflowActionButton { action_type },
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
}

fn handle_action_selection(
    mut state: ResMut<WorkflowCreationState>,
    action_buttons: Query<(&WorkflowActionButton, &Interaction), Changed<Interaction>>,
    mut commands: Commands,
    popups: Query<Entity, With<WorkflowActionPopup>>,
    step_lists: Query<(Entity, &Children), With<WorkflowStepList>>,
) {
    for (action_button, interaction) in &action_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Some(building_entity) = state.pending_building.take() else {
            continue;
        };

        let action = match action_button.action_type {
            WorkflowActionType::Pickup => WorkflowAction::Pickup(None),
            WorkflowActionType::Dropoff => WorkflowAction::Dropoff(None),
        };

        state.steps.push(WorkflowStep {
            target: building_entity,
            action,
        });

        for popup in &popups {
            commands.entity(popup).despawn();
        }

        rebuild_step_list(&mut commands, &step_lists, &state.steps);
    }
}

fn rebuild_step_list(
    commands: &mut Commands,
    step_lists: &Query<(Entity, &Children), With<WorkflowStepList>>,
    steps: &[WorkflowStep],
) {
    for (list_entity, children) in step_lists {
        for &child in children {
            commands.entity(child).despawn();
        }

        commands.entity(list_entity).with_children(|parent| {
            if steps.is_empty() {
                parent.spawn((
                    Text::new("No steps added. Click a building to add steps."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
                return;
            }

            for (i, step) in steps.iter().enumerate() {
                let action_label = match &step.action {
                    WorkflowAction::Pickup(_) => "Pickup",
                    WorkflowAction::Dropoff(_) => "Dropoff",
                };

                parent
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(24.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },
                        WorkflowStepDisplay,
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!(
                                "{}. Building {:?} - {}",
                                i + 1,
                                step.target,
                                action_label
                            )),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(TEXT_COLOR),
                        ));

                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(CANCEL_BG),
                            WorkflowStepRemoveButton { step_index: i },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("x"),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(TEXT_COLOR),
                            ));
                        });
                    });
            }
        });
    }
}

fn handle_creation_controls(
    mut state: ResMut<WorkflowCreationState>,
    confirm_buttons: Query<&Interaction, (Changed<Interaction>, With<WorkflowConfirmButton>)>,
    cancel_buttons: Query<&Interaction, (Changed<Interaction>, With<WorkflowCancelButton>)>,
    increment_buttons: Query<
        &Interaction,
        (Changed<Interaction>, With<WorkflowWorkerIncrementButton>),
    >,
    decrement_buttons: Query<
        &Interaction,
        (Changed<Interaction>, With<WorkflowWorkerDecrementButton>),
    >,
    remove_buttons: Query<(&Interaction, &WorkflowStepRemoveButton), Changed<Interaction>>,
    mut commands: Commands,
    step_lists: Query<(Entity, &Children), With<WorkflowStepList>>,
    mut create_events: MessageWriter<CreateWorkflowEvent>,
    mut next_mode: ResMut<NextState<crate::ui::UiMode>>,
) {
    let mut should_cancel = false;
    let mut should_confirm = false;
    let mut worker_delta: i32 = 0;
    let mut step_to_remove: Option<usize> = None;

    for interaction in &confirm_buttons {
        if *interaction == Interaction::Pressed {
            should_confirm = true;
        }
    }

    for interaction in &cancel_buttons {
        if *interaction == Interaction::Pressed {
            should_cancel = true;
        }
    }

    for interaction in &increment_buttons {
        if *interaction == Interaction::Pressed {
            worker_delta += 1;
        }
    }

    for interaction in &decrement_buttons {
        if *interaction == Interaction::Pressed {
            worker_delta -= 1;
        }
    }

    for (interaction, remove_btn) in &remove_buttons {
        if *interaction == Interaction::Pressed {
            step_to_remove = Some(remove_btn.step_index);
        }
    }

    if should_confirm && !state.steps.is_empty() {
        create_events.write(CreateWorkflowEvent {
            name: state.name.clone(),
            steps: state.steps.clone(),
            desired_worker_count: state.desired_worker_count,
        });
        info!(name = %state.name, steps = state.steps.len(), "workflow created");
        next_mode.set(crate::ui::UiMode::Observe);
        return;
    }

    if should_cancel {
        next_mode.set(crate::ui::UiMode::Observe);
        return;
    }

    if worker_delta != 0 {
        #[allow(clippy::cast_sign_loss)]
        let new_count =
            (i64::from(state.desired_worker_count) + i64::from(worker_delta)).clamp(1, 10) as u32;
        state.desired_worker_count = new_count;
    }

    if let Some(index) = step_to_remove {
        if index < state.steps.len() {
            state.steps.remove(index);
            rebuild_step_list(&mut commands, &step_lists, &state.steps);
        }
    }
}

fn update_worker_count_display(
    state: Res<WorkflowCreationState>,
    mut labels: Query<&mut Text, With<WorkflowWorkerCountLabel>>,
) {
    if !state.is_changed() {
        return;
    }

    for mut text in &mut labels {
        **text = format!("{}", state.desired_worker_count);
    }
}

fn update_button_hover_visuals(
    mut buttons: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&WorkflowConfirmButton>,
            Option<&WorkflowCancelButton>,
        ),
        (
            Changed<Interaction>,
            Or<(
                With<WorkflowActionButton>,
                With<WorkflowConfirmButton>,
                With<WorkflowCancelButton>,
                With<WorkflowWorkerIncrementButton>,
                With<WorkflowWorkerDecrementButton>,
                With<WorkflowStepRemoveButton>,
            )>,
        ),
    >,
) {
    for (interaction, mut bg, confirm, cancel) in &mut buttons {
        let (normal, hovered) = if confirm.is_some() {
            (CONFIRM_BG, CONFIRM_HOVER)
        } else if cancel.is_some() {
            (CANCEL_BG, CANCEL_HOVER)
        } else {
            (BUTTON_BG, BUTTON_HOVER)
        };

        *bg = match interaction {
            Interaction::Pressed | Interaction::Hovered => BackgroundColor(hovered),
            Interaction::None => BackgroundColor(normal),
        };
    }
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
                        handle_building_click_in_creation_mode,
                        handle_action_selection,
                        handle_creation_controls,
                    )
                        .in_set(UISystemSet::EntityManagement)
                        .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
                    (update_worker_count_display, update_button_hover_visuals)
                        .in_set(UISystemSet::VisualUpdates)
                        .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
                ),
            );
    }
}
