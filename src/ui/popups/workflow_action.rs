use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::ui::modes::workflow_create::{
    rebuild_step_list, WorkflowCreationState, WorkflowStepList,
};
use crate::ui::popups::building_menu::BuildingClickEvent;
use crate::{
    ui::{
        style::{ButtonStyle, BUTTON_BG, DIM_TEXT, PANEL_BORDER, POPUP_BG, TEXT_COLOR},
        UISystemSet,
    },
    workers::workflows::components::{WorkflowAction, WorkflowStep},
};

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
            ButtonStyle::default_button(),
            Hovered::default(),
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

pub struct WorkflowActionPlugin;

impl Plugin for WorkflowActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_building_click_in_creation_mode,
                handle_action_selection,
            )
                .in_set(UISystemSet::EntityManagement)
                .run_if(in_state(crate::ui::UiMode::WorkflowCreate)),
        );
    }
}
