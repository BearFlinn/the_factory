use bevy::prelude::*;

pub mod building_buttons;
pub mod building_menu;
pub mod interaction_handler;
pub mod production_display;
pub mod score_display;
pub mod sidebar;
pub mod sidebar_tabs;
pub mod spawn_worker_button;
pub mod tooltips;

pub use building_buttons::*;
pub use building_menu::*;
pub use interaction_handler::*;
pub use production_display::*;
pub use score_display::*;
pub use sidebar::*;
pub use sidebar_tabs::*;
pub use spawn_worker_button::*;
pub use tooltips::*;

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

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        configure_ui_system_sets(app);

        app.add_plugins((
            InteractionHandlerPlugin,
            SidebarPlugin,
            SidebarTabsPlugin,
            BuildingButtonsPlugin,
            ProductionDisplayPlugin,
            SpawnWorkerButtonPlugin,
            TooltipsPlugin,
            BuildingMenuPlugin,
            ScoreDisplayPlugin,
        ));
    }
}
