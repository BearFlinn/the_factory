use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::UiWidgetsPlugins;

pub mod icons;
pub mod modes;
pub mod panels;
pub mod popups;
pub mod scroll;
pub mod style;

pub use panels::action_bar::build_panel::SelectedBuilding;

use modes::workflow_builder::{FilterDropdown, TargetDropdown, WorkflowBuilderModal};
use modes::workflow_create::{WorkflowCreationPanel, WorkflowCreationState};
use panels::action_bar::build_panel::BuildingButton;
use panels::action_bar::ActivePanel;
use popups::building_menu::{BuildingMenu, CloseMenuEvent};
use scroll::handle_ui_scroll;
use style::StylePlugin;

#[derive(States, Debug, Default, Hash, PartialEq, Eq, Clone)]
pub enum UiMode {
    #[default]
    Observe,
    Place,
    WorkflowCreate,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UISystemSet {
    InputDetection,
    EntityManagement,
    VisualUpdates,
    LayoutUpdates,
}

pub fn configure_ui_system_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            UISystemSet::InputDetection,
            UISystemSet::EntityManagement,
            UISystemSet::VisualUpdates,
            UISystemSet::LayoutUpdates,
        )
            .chain()
            .in_set(crate::GameplaySet::UIUpdate),
    );
}

fn handle_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_mode: Res<State<UiMode>>,
    mut next_mode: ResMut<NextState<UiMode>>,
    menu_query: Query<Entity, With<BuildingMenu>>,
    mut close_events: MessageWriter<CloseMenuEvent>,
    mut active_panel: ResMut<ActivePanel>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    match current_mode.get() {
        UiMode::WorkflowCreate | UiMode::Place => {
            next_mode.set(UiMode::Observe);
        }
        UiMode::Observe => {
            if *active_panel == ActivePanel::None {
                for menu_entity in &menu_query {
                    close_events.write(CloseMenuEvent { menu_entity });
                }
            } else {
                *active_panel = ActivePanel::None;
            }
        }
    }
}

fn sync_selected_building_to_mode(
    selected_building: Res<SelectedBuilding>,
    current_mode: Res<State<UiMode>>,
    mut next_mode: ResMut<NextState<UiMode>>,
) {
    if !selected_building.is_changed() {
        return;
    }

    match (selected_building.building_name.as_ref(), current_mode.get()) {
        (Some(_), UiMode::Observe) => {
            next_mode.set(UiMode::Place);
        }
        (None, UiMode::Place) => {
            next_mode.set(UiMode::Observe);
        }
        _ => {}
    }
}

fn on_exit_place(
    mut commands: Commands,
    mut selected_building: ResMut<SelectedBuilding>,
    button_query: Query<Entity, (With<BuildingButton>, With<Checked>)>,
) {
    selected_building.building_name = None;
    for entity in &button_query {
        commands.entity(entity).remove::<Checked>();
    }
}

fn on_exit_workflow_create(
    mut state: ResMut<WorkflowCreationState>,
    mut commands: Commands,
    panels: Query<Entity, With<WorkflowCreationPanel>>,
    modals: Query<Entity, With<WorkflowBuilderModal>>,
    target_dropdowns: Query<Entity, With<TargetDropdown>>,
    filter_dropdowns: Query<Entity, With<FilterDropdown>>,
) {
    state.name.clear();
    state.steps.clear();
    state.desired_worker_count = 1;
    state.building_set.clear();
    state.phase = modes::workflow_create::CreationPhase::SelectBuildings;

    for entity in &panels {
        commands.entity(entity).despawn();
    }
    for entity in &modals {
        commands.entity(entity).despawn();
    }
    for entity in &target_dropdowns {
        commands.entity(entity).despawn();
    }
    for entity in &filter_dropdowns {
        commands.entity(entity).despawn();
    }
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        configure_ui_system_sets(app);

        app.init_state::<UiMode>();

        app.add_systems(
            Update,
            (
                (handle_escape, handle_ui_scroll).in_set(UISystemSet::InputDetection),
                sync_selected_building_to_mode.in_set(UISystemSet::EntityManagement),
            ),
        );

        app.add_systems(OnExit(UiMode::Place), on_exit_place);
        app.add_systems(OnExit(UiMode::WorkflowCreate), on_exit_workflow_create);

        app.add_plugins((
            InputDispatchPlugin,
            UiWidgetsPlugins,
            StylePlugin,
            icons::IconPlugin,
            modes::PlacementPlugin,
            modes::workflow_create::WorkflowCreationPlugin,
            modes::workflow_builder::WorkflowBuilderPlugin,
            panels::TopBarPlugin,
            panels::ActionBarPlugin,
            panels::action_bar::build_panel::BuildPanelPlugin,
            panels::WorkflowListPlugin,
            popups::BuildingMenuPlugin,
            popups::TooltipsPlugin,
        ));
    }
}
