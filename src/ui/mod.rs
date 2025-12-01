use bevy::prelude::*;

pub mod interaction_handler;
pub mod sidebar;
pub mod sidebar_tabs;
pub mod building_buttons;
pub mod production_display;
pub mod spawn_worker_button;
pub mod tooltips;
pub mod building_menu;

pub use interaction_handler::*;
pub use sidebar::*;
pub use sidebar_tabs::*;
pub use building_buttons::*;
pub use production_display::*;
pub use spawn_worker_button::*;
pub use tooltips::*;
pub use building_menu::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UISystemSet {
    /// Input detection and event generation
    InputDetection,
    /// Entity spawning and despawning
    EntityManagement, 
    /// Style and visual updates
    VisualUpdates,
    /// Position and layout updates
    LayoutUpdates,
}

pub fn configure_ui_system_sets(app: &mut App) {
    app.configure_sets(Update, (
        UISystemSet::InputDetection,
        UISystemSet::EntityManagement,
        UISystemSet::VisualUpdates,
        UISystemSet::LayoutUpdates,
    ).chain().in_set(crate::GameplaySet::UIUpdate));
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
            BuildingMenuPlugin
        ));
    }
}