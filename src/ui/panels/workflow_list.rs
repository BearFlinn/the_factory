use std::collections::HashMap;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::{
    ui::{
        panels::action_bar::ActivePanel,
        style::{
            ButtonStyle, ACTION_BAR_WIDTH, BUTTON_BG, CARD_BG, DIM_TEXT, HEADER_COLOR, PANEL_BG,
            PANEL_BORDER, TEXT_COLOR, TOP_BAR_HEIGHT, WARNING_COLOR,
        },
        UISystemSet,
    },
    workers::{
        workflows::components::{
            AssignWorkersEvent, BatchAssignWorkersEvent, DeleteWorkflowEvent, PauseWorkflowEvent,
            StepTarget, UnassignWorkersEvent, WaitingForItems, Workflow, WorkflowAction,
            WorkflowAssignment, WorkflowRegistry,
        },
        Worker,
    },
};

#[derive(Component)]
pub struct WorkflowPanel;

#[derive(Component)]
pub struct WorkflowListContainer;

#[derive(Component)]
pub struct WorkflowEntry {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowPauseButton {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowDeleteButton {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowFillButton {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowWorkerAddButton {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowWorkerRemoveButton {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowDetailText {
    pub workflow: Entity,
}

#[derive(Component)]
pub struct WorkflowPanelCloseButton;

pub fn spawn_workflow_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ACTION_BAR_WIDTH + 4.0),
                top: Val::Px(TOP_BAR_HEIGHT + 4.0),
                width: Val::Px(350.0),
                max_height: Val::Vh(80.0),
                min_height: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(PANEL_BORDER),
            Interaction::None,
            WorkflowPanel,
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("Workflows"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(HEADER_COLOR),
                    ));

                    header
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(24.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BUTTON_BG),
                            ButtonStyle::close(),
                            Hovered::default(),
                            WorkflowPanelCloseButton,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("X"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(TEXT_COLOR),
                            ));
                        });
                });

            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    overflow: Overflow::scroll_y(),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ScrollPosition::default(),
                WorkflowListContainer,
                crate::ui::scroll::Scrollable,
            ));
        });
}

fn handle_workflow_panel_buttons(
    mut active_panel: ResMut<ActivePanel>,
    close_buttons: Query<&Interaction, (Changed<Interaction>, With<WorkflowPanelCloseButton>)>,
    pause_buttons: Query<(&Interaction, &WorkflowPauseButton), Changed<Interaction>>,
    delete_buttons: Query<(&Interaction, &WorkflowDeleteButton), Changed<Interaction>>,
    add_buttons: Query<(&Interaction, &WorkflowWorkerAddButton), Changed<Interaction>>,
    remove_buttons: Query<(&Interaction, &WorkflowWorkerRemoveButton), Changed<Interaction>>,
    fill_buttons: Query<(&Interaction, &WorkflowFillButton), Changed<Interaction>>,
    mut pause_events: MessageWriter<PauseWorkflowEvent>,
    mut delete_events: MessageWriter<DeleteWorkflowEvent>,
    mut assign_events: MessageWriter<AssignWorkersEvent>,
    mut unassign_events: MessageWriter<UnassignWorkersEvent>,
    mut batch_assign_events: MessageWriter<BatchAssignWorkersEvent>,
    idle_workers: Query<Entity, (With<Worker>, Without<WorkflowAssignment>)>,
    assigned_workers: Query<(Entity, &WorkflowAssignment), With<Worker>>,
    workflows: Query<&Workflow>,
) {
    for interaction in &close_buttons {
        if *interaction == Interaction::Pressed {
            *active_panel = ActivePanel::None;
            return;
        }
    }

    for (interaction, btn) in &pause_buttons {
        if *interaction == Interaction::Pressed {
            pause_events.write(PauseWorkflowEvent {
                workflow: btn.workflow,
            });
        }
    }

    for (interaction, btn) in &delete_buttons {
        if *interaction == Interaction::Pressed {
            delete_events.write(DeleteWorkflowEvent {
                workflow: btn.workflow,
            });
        }
    }

    for (interaction, btn) in &fill_buttons {
        if *interaction == Interaction::Pressed {
            if let Ok(workflow) = workflows.get(btn.workflow) {
                batch_assign_events.write(BatchAssignWorkersEvent {
                    workflow: btn.workflow,
                    count: workflow.desired_worker_count,
                });
            }
        }
    }

    for (interaction, btn) in &add_buttons {
        if *interaction == Interaction::Pressed {
            if let Some(worker) = idle_workers.iter().next() {
                assign_events.write(AssignWorkersEvent {
                    workflow: btn.workflow,
                    workers: vec![worker],
                });
            }
        }
    }

    for (interaction, btn) in &remove_buttons {
        if *interaction == Interaction::Pressed {
            let worker = assigned_workers
                .iter()
                .find(|(_, assignment)| assignment.workflow == btn.workflow)
                .map(|(entity, _)| entity);

            if let Some(worker_entity) = worker {
                unassign_events.write(UnassignWorkersEvent {
                    workers: vec![worker_entity],
                });
            }
        }
    }
}

fn update_workflow_panel_content(
    mut commands: Commands,
    list_containers: Query<Entity, With<WorkflowListContainer>>,
    registry: Res<WorkflowRegistry>,
    workflows: Query<&Workflow>,
    assigned_workers: Query<(&WorkflowAssignment, Has<WaitingForItems>), With<Worker>>,
    names: Query<&Name>,
) {
    for container in &list_containers {
        commands.entity(container).despawn_related::<Children>();

        if registry.workflows.is_empty() {
            commands.entity(container).with_children(|parent| {
                parent.spawn((
                    Text::new("No workflows. Press N to create one."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(DIM_TEXT),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });
            continue;
        }

        commands.entity(container).with_children(|parent| {
            for &workflow_entity in &registry.workflows {
                let Ok(workflow) = workflows.get(workflow_entity) else {
                    continue;
                };

                let mut current_workers = 0u32;
                let mut waiting_workers = 0u32;
                for (assignment, is_waiting) in &assigned_workers {
                    if assignment.workflow == workflow_entity {
                        current_workers += 1;
                        if is_waiting {
                            waiting_workers += 1;
                        }
                    }
                }

                spawn_workflow_card(
                    parent,
                    workflow_entity,
                    workflow,
                    current_workers,
                    waiting_workers,
                    &names,
                );
            }
        });
    }
}

fn spawn_workflow_card(
    parent: &mut ChildSpawnerCommands,
    workflow_entity: Entity,
    workflow: &Workflow,
    current_workers: u32,
    waiting_workers: u32,
    names: &Query<&Name>,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(CARD_BG),
            BorderColor::all(PANEL_BORDER),
            WorkflowEntry {
                workflow: workflow_entity,
            },
        ))
        .with_children(|card| {
            spawn_card_header(card, workflow);
            spawn_card_details(
                card,
                workflow_entity,
                workflow,
                current_workers,
                waiting_workers,
                names,
            );
            spawn_card_buttons(card, workflow_entity, workflow.is_paused);
        });
}

fn spawn_card_header(card: &mut ChildSpawnerCommands, workflow: &Workflow) {
    card.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Text::new(&workflow.name),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(HEADER_COLOR),
        ));

        if workflow.is_paused {
            row.spawn((
                Text::new("[PAUSED]"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.7, 0.2)),
            ));
        }
    });
}

fn spawn_card_details(
    card: &mut ChildSpawnerCommands,
    workflow_entity: Entity,
    workflow: &Workflow,
    current_workers: u32,
    waiting_workers: u32,
    names: &Query<&Name>,
) {
    let pool_summary = build_pool_summary(&workflow.building_set, names);
    card.spawn((
        Text::new(format!("Buildings: {pool_summary}")),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(DIM_TEXT),
    ));

    let step_details: Vec<String> = workflow
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let action_label = match &step.action {
                WorkflowAction::Pickup(_) => "Pickup from",
                WorkflowAction::Dropoff(_) => "Dropoff to",
            };
            let target_label = match &step.target {
                StepTarget::Specific(entity) => names
                    .get(*entity)
                    .map_or_else(|_| "???".to_string(), |n| n.as_str().to_string()),
                StepTarget::ByType(type_name) => format!("any {type_name}"),
            };
            format!("  {}. {} {}", i + 1, action_label, target_label)
        })
        .collect();

    card.spawn((
        Text::new(format!(
            "Steps: {}\n{}",
            workflow.steps.len(),
            step_details.join("\n")
        )),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(DIM_TEXT),
        WorkflowDetailText {
            workflow: workflow_entity,
        },
    ));

    let worker_color = if current_workers >= workflow.desired_worker_count {
        Color::srgb(0.3, 0.8, 0.3)
    } else if waiting_workers > 0 {
        WARNING_COLOR
    } else {
        TEXT_COLOR
    };

    let worker_text = if waiting_workers > 0 {
        format!(
            "Workers: {current_workers}/{} ({waiting_workers} waiting)",
            workflow.desired_worker_count
        )
    } else {
        format!(
            "Workers: {current_workers}/{}",
            workflow.desired_worker_count
        )
    };

    card.spawn((
        Text::new(worker_text),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(worker_color),
    ));
}

fn build_pool_summary(
    building_set: &std::collections::HashSet<Entity>,
    names: &Query<&Name>,
) -> String {
    if building_set.is_empty() {
        return "None".to_string();
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
    types
        .iter()
        .map(|(name, count)| {
            if *count > 1 {
                format!("{count}x {name}")
            } else {
                name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn spawn_card_buttons(card: &mut ChildSpawnerCommands, workflow_entity: Entity, is_paused: bool) {
    card.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(4.0),
        margin: UiRect::top(Val::Px(2.0)),
        ..default()
    })
    .with_children(|button_row| {
        let pause_label = if is_paused { "Resume" } else { "Pause" };

        spawn_panel_button(
            button_row,
            pause_label,
            ButtonStyle::confirm(),
            WorkflowPauseButton {
                workflow: workflow_entity,
            },
        );
        spawn_panel_button(
            button_row,
            "Delete",
            ButtonStyle::cancel(),
            WorkflowDeleteButton {
                workflow: workflow_entity,
            },
        );
        spawn_panel_button(
            button_row,
            "Fill",
            ButtonStyle::default_button(),
            WorkflowFillButton {
                workflow: workflow_entity,
            },
        );
        spawn_panel_button(
            button_row,
            "+W",
            ButtonStyle::default_button(),
            WorkflowWorkerAddButton {
                workflow: workflow_entity,
            },
        );
        spawn_panel_button(
            button_row,
            "-W",
            ButtonStyle::default_button(),
            WorkflowWorkerRemoveButton {
                workflow: workflow_entity,
            },
        );
    });
}

fn spawn_panel_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    style: ButtonStyle,
    marker: impl Component,
) {
    parent
        .spawn((
            Button,
            Node {
                height: Val::Px(26.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(style.default_bg),
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

pub struct WorkflowListPlugin;

impl Plugin for WorkflowListPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_workflow_panel_buttons.in_set(UISystemSet::EntityManagement),
                (update_workflow_panel_content,)
                    .in_set(UISystemSet::VisualUpdates)
                    .run_if(|active: Res<ActivePanel>| *active == ActivePanel::Workflows),
            ),
        );
    }
}
