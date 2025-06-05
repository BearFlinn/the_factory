use bevy::prelude::*;

pub mod interaction_handler;
pub mod sidebar;
pub mod sidebar_tabs;
pub mod building_buttons;
pub mod production_display;
pub mod spawn_worker_button;

pub use interaction_handler::*;
pub use sidebar::*;
pub use sidebar_tabs::*;
pub use building_buttons::*;
pub use production_display::*;
pub use spawn_worker_button::*;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                InteractionHandlerPlugin,
                SidebarPlugin,
                SidebarTabsPlugin,
                BuildingButtonsPlugin,
                ProductionDisplayPlugin,
                SpawnWorkerButtonPlugin
            ));
    }
}