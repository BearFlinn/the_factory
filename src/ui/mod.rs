use bevy::prelude::*;

pub mod interaction_handler;
pub mod sidebar;
pub mod sidebar_tabs;
pub mod building_buttons;

pub use interaction_handler::*;
pub use sidebar::*;
pub use sidebar_tabs::*;
pub use building_buttons::*;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((InteractionHandlerPlugin, SidebarPlugin, SidebarTabsPlugin, BuildingButtonsPlugin));
    }
}