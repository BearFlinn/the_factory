use std::collections::{HashMap, HashSet};

use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::{
    grid::Position,
    materials::ItemRegistry,
    ui::{
        modes::workflow_create::{CreationPhase, WorkflowCreationState},
        scroll::Scrollable,
        style::{
            ButtonStyle, BUTTON_BG, CANCEL_BG, CONFIRM_BG, DIM_TEXT, HEADER_COLOR, PANEL_BG,
            PANEL_BORDER, POPUP_BG, SELECTED_BG, TEXT_COLOR,
        },
        UISystemSet,
    },
    workers::workflows::components::{
        CreateWorkflowEvent, StepTarget, WorkflowAction, WorkflowStep,
    },
};

#[derive(Component)]
pub struct WorkflowBuilderModal;

#[derive(Component)]
pub struct BuilderStepList;

#[derive(Component)]
pub struct BuilderStepRow {
    pub step_index: usize,
}

#[derive(Component)]
pub struct StepActionButton {
    pub step_index: usize,
}

#[derive(Component)]
pub struct StepTargetButton {
    pub step_index: usize,
}

#[derive(Component)]
pub struct StepFilterButton {
    pub step_index: usize,
}

#[derive(Component)]
pub struct StepRemoveButton {
    pub step_index: usize,
}

#[derive(Component)]
pub struct AddStepButton;

#[derive(Component)]
pub struct BuilderSaveButton;

#[derive(Component)]
pub struct BuilderCancelButton;

#[derive(Component)]
pub struct BuilderBackButton;

#[derive(Component)]
pub struct BuilderWorkerCountLabel;

#[derive(Component)]
pub struct BuilderWorkerIncrementButton;

#[derive(Component)]
pub struct BuilderWorkerDecrementButton;

#[derive(Component)]
pub struct TargetDropdown {
    pub step_index: usize,
}

#[derive(Component)]
pub struct TargetDropdownOption {
    pub step_index: usize,
    pub target: StepTarget,
}

#[derive(Component)]
pub struct FilterDropdown {
    pub step_index: usize,
}

#[derive(Component)]
pub struct FilterCheckbox {
    pub step_index: usize,
    pub item_name: String,
}

#[derive(Component)]
pub struct BuilderPoolSummary;

fn spawn_builder_modal_on_phase(
    state: Res<WorkflowCreationState>,
    mut commands: Commands,
    existing_modals: Query<Entity, With<WorkflowBuilderModal>>,
    names: Query<&Name>,
) {
    if !state.is_changed() {
        return;
    }
    if state.phase != CreationPhase::BuilderModal {
        return;
    }
    if !existing_modals.is_empty() {
        return;
    }

    spawn_builder_modal(&mut commands, &state, &names);
}

fn spawn_builder_modal(
    commands: &mut Commands,
    state: &WorkflowCreationState,
    names: &Query<&Name>,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            Interaction::None,
            WorkflowBuilderModal,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(600.0),
                        max_height: Val::Vh(80.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(PANEL_BORDER),
                    ScrollPosition::default(),
                    Scrollable,
                ))
                .with_children(|modal| {
                    spawn_modal_header(modal, &state.name);
                    spawn_pool_summary(modal, &state.building_set, names);
                    spawn_step_section(modal, state, names);
                    spawn_worker_count_section(modal, state.desired_worker_count);
                    spawn_modal_buttons(modal);
                });
        });
}

fn spawn_modal_header(parent: &mut ChildSpawnerCommands, name: &str) {
    parent.spawn((
        Text::new(format!("Workflow Builder: \"{name}\"")),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(HEADER_COLOR),
    ));
}

fn spawn_pool_summary(
    parent: &mut ChildSpawnerCommands,
    building_set: &HashSet<Entity>,
    names: &Query<&Name>,
) {
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

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(PANEL_BORDER),
            BuilderPoolSummary,
        ))
        .with_children(|section| {
            section.spawn((
                Text::new(format!("Building Pool: {summary}")),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(DIM_TEXT),
            ));
        });
}

fn spawn_step_section(
    parent: &mut ChildSpawnerCommands,
    state: &WorkflowCreationState,
    names: &Query<&Name>,
) {
    parent.spawn((
        Text::new("Steps:"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(TEXT_COLOR),
    ));

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                min_height: Val::Px(30.0),
                ..default()
            },
            BuilderStepList,
        ))
        .with_children(|step_list| {
            if state.steps.is_empty() {
                step_list.spawn((
                    Text::new("No steps. Click '+ Add Step' to begin."),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
            } else {
                for (i, step) in state.steps.iter().enumerate() {
                    spawn_step_row(step_list, i, step, &state.building_set, names);
                }
            }
        });

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_BG),
            ButtonStyle::default_button(),
            Hovered::default(),
            AddStepButton,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("+ Add Step"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
}

fn step_labels(step: &WorkflowStep, names: &Query<&Name>) -> (String, String, String, String) {
    let action = match &step.action {
        WorkflowAction::Pickup(_) => "Pickup",
        WorkflowAction::Dropoff(_) => "Dropoff",
    }
    .to_string();
    let preposition = match &step.action {
        WorkflowAction::Pickup(_) => "from",
        WorkflowAction::Dropoff(_) => "to",
    }
    .to_string();
    let target = match &step.target {
        StepTarget::Specific(entity) => names
            .get(*entity)
            .map_or_else(|_| "Unknown".to_string(), |n| n.as_str().to_string()),
        StepTarget::ByType(type_name) => format!("any {type_name}"),
    };
    let filter = match &step.action {
        WorkflowAction::Pickup(Some(items)) | WorkflowAction::Dropoff(Some(items)) => {
            if items.is_empty() {
                "All".to_string()
            } else {
                let keys: Vec<_> = items.keys().collect();
                if keys.len() <= 2 {
                    keys.iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    format!("{} items", keys.len())
                }
            }
        }
        _ => "All".to_string(),
    };
    (action, preposition, target, filter)
}

fn spawn_step_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    step: &WorkflowStep,
    _building_set: &HashSet<Entity>,
    names: &Query<&Name>,
) {
    let (action_label, preposition, target_label, filter_label) = step_labels(step, names);

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                ..default()
            },
            BuilderStepRow { step_index: index },
        ))
        .with_children(|row| {
            spawn_step_row_children(
                row,
                index,
                &action_label,
                &preposition,
                &target_label,
                &filter_label,
            );
        });
}

fn spawn_step_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    width: Val,
    style: ButtonStyle,
    marker: impl Component,
) {
    let bg = style.default_bg;
    parent
        .spawn((
            Button,
            Node {
                width,
                height: Val::Px(26.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            style,
            Hovered::default(),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
}

fn spawn_step_row_children(
    row: &mut ChildSpawnerCommands,
    index: usize,
    action_label: &str,
    preposition: &str,
    target_label: &str,
    filter_label: &str,
) {
    row.spawn((
        Text::new(format!("{}.", index + 1)),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(DIM_TEXT),
        Node {
            width: Val::Px(20.0),
            ..default()
        },
    ));

    spawn_step_button(
        row,
        action_label,
        Val::Px(70.0),
        ButtonStyle::default_button(),
        StepActionButton { step_index: index },
    );

    row.spawn((
        Text::new(preposition),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(DIM_TEXT),
    ));

    spawn_step_button(
        row,
        target_label,
        Val::Px(140.0),
        ButtonStyle::default_button(),
        StepTargetButton { step_index: index },
    );

    spawn_step_button(
        row,
        filter_label,
        Val::Px(60.0),
        ButtonStyle::default_button(),
        StepFilterButton { step_index: index },
    );

    spawn_step_button(
        row,
        "x",
        Val::Px(24.0),
        ButtonStyle::cancel(),
        StepRemoveButton { step_index: index },
    );
}

fn spawn_worker_count_section(parent: &mut ChildSpawnerCommands, count: u32) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::vertical(Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(PANEL_BORDER),
        ))
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
                ButtonStyle::default_button(),
                Hovered::default(),
                BuilderWorkerDecrementButton,
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
                BuilderWorkerCountLabel,
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
                ButtonStyle::default_button(),
                Hovered::default(),
                BuilderWorkerIncrementButton,
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

fn spawn_modal_buttons(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(34.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                padding: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            BorderColor::all(PANEL_BORDER),
        ))
        .with_children(|row| {
            row.spawn((
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(CANCEL_BG),
                ButtonStyle::cancel(),
                Hovered::default(),
                BuilderCancelButton,
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

            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|right| {
                right
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BUTTON_BG),
                        ButtonStyle::default_button(),
                        Hovered::default(),
                        BuilderBackButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("<- Back to Pool"),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(TEXT_COLOR),
                        ));
                    });

                right
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(CONFIRM_BG),
                        ButtonStyle::confirm(),
                        Hovered::default(),
                        BuilderSaveButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Save"),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(TEXT_COLOR),
                        ));
                    });
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_builder_controls(
    mut state: ResMut<WorkflowCreationState>,
    save_buttons: Query<&Interaction, (Changed<Interaction>, With<BuilderSaveButton>)>,
    cancel_buttons: Query<&Interaction, (Changed<Interaction>, With<BuilderCancelButton>)>,
    back_buttons: Query<&Interaction, (Changed<Interaction>, With<BuilderBackButton>)>,
    add_step_buttons: Query<&Interaction, (Changed<Interaction>, With<AddStepButton>)>,
    remove_buttons: Query<(&Interaction, &StepRemoveButton), Changed<Interaction>>,
    increment_buttons: Query<
        &Interaction,
        (Changed<Interaction>, With<BuilderWorkerIncrementButton>),
    >,
    decrement_buttons: Query<
        &Interaction,
        (Changed<Interaction>, With<BuilderWorkerDecrementButton>),
    >,
    mut commands: Commands,
    modals: Query<Entity, With<WorkflowBuilderModal>>,
    mut create_events: MessageWriter<CreateWorkflowEvent>,
    mut next_mode: ResMut<NextState<crate::ui::UiMode>>,
    names: Query<&Name>,
    step_lists: Query<(Entity, &Children), With<BuilderStepList>>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for interaction in &save_buttons {
        if *interaction == Interaction::Pressed && !state.steps.is_empty() {
            create_events.write(CreateWorkflowEvent {
                name: state.name.clone(),
                building_set: state.building_set.clone(),
                steps: state.steps.clone(),
                desired_worker_count: state.desired_worker_count,
            });
            info!(name = %state.name, steps = state.steps.len(), "workflow created");
            for entity in &modals {
                commands.entity(entity).despawn();
            }
            next_mode.set(crate::ui::UiMode::Observe);
            return;
        }
    }

    for interaction in &cancel_buttons {
        if *interaction == Interaction::Pressed {
            for entity in &modals {
                commands.entity(entity).despawn();
            }
            next_mode.set(crate::ui::UiMode::Observe);
            return;
        }
    }

    for interaction in &back_buttons {
        if *interaction == Interaction::Pressed {
            state.phase = CreationPhase::SelectBuildings;
            for entity in &modals {
                commands.entity(entity).despawn();
            }
            return;
        }
    }

    for interaction in &add_step_buttons {
        if *interaction == Interaction::Pressed {
            let default_target = get_first_building_type(&state.building_set, &names);
            state.steps.push(WorkflowStep {
                target: default_target,
                action: WorkflowAction::Pickup(None),
            });
            rebuild_modal_steps(&mut commands, &step_lists, &state, &names);
            return;
        }
    }

    let mut step_removed = false;
    for (interaction, remove_btn) in &remove_buttons {
        if *interaction == Interaction::Pressed && remove_btn.step_index < state.steps.len() {
            state.steps.remove(remove_btn.step_index);
            step_removed = true;
        }
    }
    if step_removed {
        rebuild_modal_steps(&mut commands, &step_lists, &state, &names);
        return;
    }

    let mut worker_delta: i32 = 0;
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
    if worker_delta != 0 {
        #[allow(clippy::cast_sign_loss)]
        let new_count =
            (i64::from(state.desired_worker_count) + i64::from(worker_delta)).clamp(1, 10) as u32;
        state.desired_worker_count = new_count;
    }
}

fn handle_step_action_toggle(
    mut state: ResMut<WorkflowCreationState>,
    action_buttons: Query<(&Interaction, &StepActionButton), Changed<Interaction>>,
    mut commands: Commands,
    step_lists: Query<(Entity, &Children), With<BuilderStepList>>,
    names: Query<&Name>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for (interaction, btn) in &action_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(step) = state.steps.get_mut(btn.step_index) {
            step.action = match &step.action {
                WorkflowAction::Pickup(filter) => WorkflowAction::Dropoff(filter.clone()),
                WorkflowAction::Dropoff(filter) => WorkflowAction::Pickup(filter.clone()),
            };
            rebuild_modal_steps(&mut commands, &step_lists, &state, &names);
            return;
        }
    }
}

fn handle_step_target_button(
    state: Res<WorkflowCreationState>,
    target_buttons: Query<
        (&Interaction, &StepTargetButton, &GlobalTransform),
        Changed<Interaction>,
    >,
    mut commands: Commands,
    existing_dropdowns: Query<Entity, With<TargetDropdown>>,
    names: Query<&Name>,
    positions: Query<&Position>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for (interaction, btn, transform) in &target_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        for entity in &existing_dropdowns {
            commands.entity(entity).despawn();
        }

        let mut types: HashMap<String, Vec<(Entity, Option<(i32, i32)>)>> = HashMap::new();
        for &entity in &state.building_set {
            let name = names
                .get(entity)
                .map_or_else(|_| "Unknown".to_string(), |n| n.as_str().to_string());
            let pos = positions.get(entity).ok().map(|p| (p.x, p.y));
            types.entry(name).or_default().push((entity, pos));
        }

        let mut sorted_types: Vec<_> = types.into_iter().collect();
        sorted_types.sort_by(|a, b| a.0.cmp(&b.0));

        let button_pos = transform.translation();

        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(button_pos.x),
                    top: Val::Px(button_pos.y + 28.0),
                    width: Val::Px(220.0),
                    max_height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    row_gap: Val::Px(2.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(POPUP_BG),
                BorderColor::all(PANEL_BORDER),
                ScrollPosition::default(),
                Scrollable,
                TargetDropdown {
                    step_index: btn.step_index,
                },
                GlobalZIndex(100),
            ))
            .with_children(|dropdown| {
                for (type_name, buildings) in &sorted_types {
                    spawn_dropdown_option(
                        dropdown,
                        &format!("any {type_name}"),
                        btn.step_index,
                        StepTarget::ByType(type_name.clone()),
                    );

                    for (entity, pos) in buildings {
                        let label = match pos {
                            Some((x, y)) => format!("{type_name} at ({x},{y})"),
                            None => type_name.clone(),
                        };
                        spawn_dropdown_option(
                            dropdown,
                            &label,
                            btn.step_index,
                            StepTarget::Specific(*entity),
                        );
                    }
                }
            });
    }
}

fn spawn_dropdown_option(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    step_index: usize,
    target: StepTarget,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BUTTON_BG),
            ButtonStyle::default_button(),
            Hovered::default(),
            TargetDropdownOption { step_index, target },
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        });
}

fn handle_target_dropdown_selection(
    mut state: ResMut<WorkflowCreationState>,
    options: Query<(&Interaction, &TargetDropdownOption), Changed<Interaction>>,
    mut commands: Commands,
    dropdowns: Query<Entity, With<TargetDropdown>>,
    step_lists: Query<(Entity, &Children), With<BuilderStepList>>,
    names: Query<&Name>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for (interaction, option) in &options {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Some(step) = state.steps.get_mut(option.step_index) {
            step.target = option.target.clone();
        }

        for entity in &dropdowns {
            commands.entity(entity).despawn();
        }

        rebuild_modal_steps(&mut commands, &step_lists, &state, &names);
        return;
    }
}

fn handle_step_filter_button(
    state: Res<WorkflowCreationState>,
    filter_buttons: Query<
        (&Interaction, &StepFilterButton, &GlobalTransform),
        Changed<Interaction>,
    >,
    mut commands: Commands,
    existing_dropdowns: Query<Entity, Or<(With<FilterDropdown>, With<TargetDropdown>)>>,
    item_registry: Res<ItemRegistry>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for (interaction, btn, transform) in &filter_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        for entity in &existing_dropdowns {
            commands.entity(entity).despawn();
        }

        let current_filter = state
            .steps
            .get(btn.step_index)
            .and_then(|step| match &step.action {
                WorkflowAction::Pickup(filter) | WorkflowAction::Dropoff(filter) => filter.clone(),
            });

        let selected_items: HashSet<String> = current_filter
            .as_ref()
            .map(|f| f.keys().cloned().collect())
            .unwrap_or_default();

        let button_pos = transform.translation();

        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(button_pos.x),
                    top: Val::Px(button_pos.y + 28.0),
                    width: Val::Px(200.0),
                    max_height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    row_gap: Val::Px(2.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(POPUP_BG),
                BorderColor::all(PANEL_BORDER),
                ScrollPosition::default(),
                Scrollable,
                FilterDropdown {
                    step_index: btn.step_index,
                },
                GlobalZIndex(100),
            ))
            .with_children(|dropdown| {
                dropdown.spawn((
                    Text::new("Item Filter (empty = all):"),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                    Node {
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                ));

                let mut item_names: Vec<_> = item_registry.definitions.keys().cloned().collect();
                item_names.sort();

                for item_name in item_names {
                    let is_selected = selected_items.contains(&item_name);
                    let label = if is_selected {
                        format!("[x] {item_name}")
                    } else {
                        format!("[ ] {item_name}")
                    };

                    dropdown
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(22.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(if is_selected { SELECTED_BG } else { BUTTON_BG }),
                            ButtonStyle::default_button(),
                            Hovered::default(),
                            FilterCheckbox {
                                step_index: btn.step_index,
                                item_name: item_name.clone(),
                            },
                        ))
                        .with_children(|checkbox_btn| {
                            checkbox_btn.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(TEXT_COLOR),
                            ));
                        });
                }
            });
    }
}

fn handle_filter_checkbox_toggle(
    mut state: ResMut<WorkflowCreationState>,
    checkboxes: Query<(&Interaction, &FilterCheckbox), Changed<Interaction>>,
    mut commands: Commands,
    filter_dropdowns: Query<Entity, With<FilterDropdown>>,
    step_lists: Query<(Entity, &Children), With<BuilderStepList>>,
    names: Query<&Name>,
) {
    if state.phase != CreationPhase::BuilderModal {
        return;
    }

    for (interaction, checkbox) in &checkboxes {
        if *interaction != Interaction::Pressed {
            continue;
        }

        if let Some(step) = state.steps.get_mut(checkbox.step_index) {
            let filter = match &mut step.action {
                WorkflowAction::Pickup(filter) | WorkflowAction::Dropoff(filter) => filter,
            };

            let mut items = filter.take().unwrap_or_default();
            if items.contains_key(&checkbox.item_name) {
                items.remove(&checkbox.item_name);
            } else {
                items.insert(checkbox.item_name.clone(), u32::MAX);
            }

            if items.is_empty() {
                *filter = None;
            } else {
                *filter = Some(items);
            }
        }

        for entity in &filter_dropdowns {
            commands.entity(entity).despawn();
        }
        rebuild_modal_steps(&mut commands, &step_lists, &state, &names);
        return;
    }
}

fn get_first_building_type(building_set: &HashSet<Entity>, names: &Query<&Name>) -> StepTarget {
    let mut type_name = None;
    for &entity in building_set {
        if let Ok(name) = names.get(entity) {
            type_name = Some(name.as_str().to_string());
            break;
        }
    }
    match type_name {
        Some(name) => StepTarget::ByType(name),
        None => StepTarget::ByType("Unknown".to_string()),
    }
}

fn rebuild_modal_steps(
    commands: &mut Commands,
    step_lists: &Query<(Entity, &Children), With<BuilderStepList>>,
    state: &WorkflowCreationState,
    names: &Query<&Name>,
) {
    for (list_entity, children) in step_lists {
        for &child in children {
            commands.entity(child).despawn();
        }

        commands.entity(list_entity).with_children(|parent| {
            if state.steps.is_empty() {
                parent.spawn((
                    Text::new("No steps. Click '+ Add Step' to begin."),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                ));
                return;
            }

            for (i, step) in state.steps.iter().enumerate() {
                spawn_step_row(parent, i, step, &state.building_set, names);
            }
        });
    }
}

fn update_builder_worker_count(
    state: Res<WorkflowCreationState>,
    mut labels: Query<&mut Text, With<BuilderWorkerCountLabel>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut text in &mut labels {
        **text = format!("{}", state.desired_worker_count);
    }
}

fn close_dropdowns_on_outside_click(
    interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            Without<TargetDropdownOption>,
            Without<FilterCheckbox>,
            Without<StepTargetButton>,
            Without<StepFilterButton>,
        ),
    >,
    mut commands: Commands,
    target_dropdowns: Query<Entity, With<TargetDropdown>>,
    filter_dropdowns: Query<Entity, With<FilterDropdown>>,
) {
    if target_dropdowns.is_empty() && filter_dropdowns.is_empty() {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            for entity in &target_dropdowns {
                commands.entity(entity).despawn();
            }
            for entity in &filter_dropdowns {
                commands.entity(entity).despawn();
            }
            return;
        }
    }
}

pub struct WorkflowBuilderPlugin;

impl Plugin for WorkflowBuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_builder_modal_on_phase
                    .in_set(UISystemSet::EntityManagement)
                    .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
                (
                    handle_builder_controls,
                    handle_step_action_toggle,
                    handle_step_target_button,
                    handle_target_dropdown_selection,
                    handle_step_filter_button,
                    handle_filter_checkbox_toggle,
                    close_dropdowns_on_outside_click,
                )
                    .in_set(UISystemSet::EntityManagement)
                    .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
                update_builder_worker_count
                    .in_set(UISystemSet::VisualUpdates)
                    .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
            ),
        );
    }
}
