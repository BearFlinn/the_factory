use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::{
    ui::{
        style::{
            ButtonStyle, BUTTON_BG, CARD_BG, DIM_TEXT, HEADER_COLOR, PANEL_BG, PANEL_BORDER,
            TEXT_COLOR,
        },
        UISystemSet,
    },
    workers::{
        workflows::components::{
            AssignWorkersEvent, DeleteWorkflowEvent, PauseWorkflowEvent, UnassignWorkersEvent,
            Workflow, WorkflowAction, WorkflowAssignment, WorkflowRegistry,
        },
        Worker,
    },
};

#[derive(Resource, Default)]
pub struct WorkflowPanelState {
    pub visible: bool,
}

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

fn toggle_workflow_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<WorkflowPanelState>,
    mut commands: Commands,
    panels: Query<Entity, With<WorkflowPanel>>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }

    state.visible = !state.visible;

    if state.visible {
        spawn_panel(&mut commands);
    } else {
        for entity in &panels {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(100.0),
                width: Val::Px(350.0),
                height: Val::Vh(70.0),
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
            ));
        });
}

fn handle_workflow_panel_buttons(
    mut state: ResMut<WorkflowPanelState>,
    mut commands: Commands,
    panels: Query<Entity, With<WorkflowPanel>>,
    close_buttons: Query<&Interaction, (Changed<Interaction>, With<WorkflowPanelCloseButton>)>,
    pause_buttons: Query<(&Interaction, &WorkflowPauseButton), Changed<Interaction>>,
    delete_buttons: Query<(&Interaction, &WorkflowDeleteButton), Changed<Interaction>>,
    add_buttons: Query<(&Interaction, &WorkflowWorkerAddButton), Changed<Interaction>>,
    remove_buttons: Query<(&Interaction, &WorkflowWorkerRemoveButton), Changed<Interaction>>,
    mut pause_events: MessageWriter<PauseWorkflowEvent>,
    mut delete_events: MessageWriter<DeleteWorkflowEvent>,
    mut assign_events: MessageWriter<AssignWorkersEvent>,
    mut unassign_events: MessageWriter<UnassignWorkersEvent>,
    idle_workers: Query<Entity, (With<Worker>, Without<WorkflowAssignment>)>,
    assigned_workers: Query<(Entity, &WorkflowAssignment), With<Worker>>,
) {
    for interaction in &close_buttons {
        if *interaction == Interaction::Pressed {
            state.visible = false;
            for entity in &panels {
                commands.entity(entity).despawn();
            }
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
    assigned_workers: Query<&WorkflowAssignment, With<Worker>>,
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

                let current_workers = assigned_workers
                    .iter()
                    .filter(|a| a.workflow == workflow_entity)
                    .count();

                spawn_workflow_card(parent, workflow_entity, workflow, current_workers);
            }
        });
    }
}

fn spawn_workflow_card(
    parent: &mut ChildSpawnerCommands,
    workflow_entity: Entity,
    workflow: &Workflow,
    current_workers: usize,
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
            spawn_card_details(card, workflow_entity, workflow, current_workers);
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
    current_workers: usize,
) {
    let step_details: Vec<String> = workflow
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let action_label = match &step.action {
                WorkflowAction::Pickup(_) => "Pickup",
                WorkflowAction::Dropoff(_) => "Dropoff",
            };
            format!("  {}. {}", i + 1, action_label)
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

    card.spawn((
        Text::new(format!(
            "Workers: {current_workers}/{}",
            workflow.desired_worker_count
        )),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(TEXT_COLOR),
    ));
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

const LINE_HEIGHT: f32 = 21.0;
const SCROLL_GAP: f32 = 6.0;

fn handle_workflow_scroll(
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    panel_query: Query<(&GlobalTransform, &ComputedNode), With<WorkflowPanel>>,
    mut scroll_query: Query<
        (&mut ScrollPosition, &ComputedNode, &Node, &Children),
        With<WorkflowListContainer>,
    >,
    child_sizes: Query<&ComputedNode, Without<WorkflowListContainer>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let cursor_over_panel = panel_query.iter().any(|(transform, node)| {
        let center = transform.translation().truncate();
        let half = node.size() / 2.0;
        cursor_pos.x >= center.x - half.x
            && cursor_pos.x <= center.x + half.x
            && cursor_pos.y >= center.y - half.y
            && cursor_pos.y <= center.y + half.y
    });

    if !cursor_over_panel {
        return;
    }

    for scroll in mouse_wheel.read() {
        let delta = match scroll.unit {
            MouseScrollUnit::Line => scroll.y * LINE_HEIGHT,
            MouseScrollUnit::Pixel => scroll.y,
        };

        for (mut scroll_pos, container_node, container_style, children) in &mut scroll_query {
            let content_height: f32 = children
                .iter()
                .filter_map(|child| child_sizes.get(child).ok())
                .map(|node| node.size().y)
                .sum();
            let gap = match container_style.row_gap {
                Val::Px(px) => px,
                _ => SCROLL_GAP,
            };
            #[allow(clippy::cast_precision_loss)]
            let gap_total = children.len().saturating_sub(1) as f32 * gap;
            let total_content = content_height + gap_total;
            let max_offset = (total_content - container_node.size().y).max(0.0);
            scroll_pos.y = (scroll_pos.y - delta).clamp(0.0, max_offset);
        }
    }
}

pub struct WorkflowPanelPlugin;

impl Plugin for WorkflowPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorkflowPanelState>().add_systems(
            Update,
            (
                (
                    toggle_workflow_panel,
                    handle_workflow_scroll.run_if(|state: Res<WorkflowPanelState>| state.visible),
                )
                    .in_set(UISystemSet::InputDetection),
                handle_workflow_panel_buttons.in_set(UISystemSet::EntityManagement),
                (update_workflow_panel_content,)
                    .in_set(UISystemSet::VisualUpdates)
                    .run_if(|state: Res<WorkflowPanelState>| state.visible),
            ),
        );
    }
}
