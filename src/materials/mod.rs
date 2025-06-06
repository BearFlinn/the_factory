use bevy::prelude::*;

pub mod items;
pub mod recipes;

pub use items::*;
pub use recipes::*;

pub struct MaterialsPlugin;

pub fn setup(mut commands: Commands) {
    commands.insert_resource(ItemRegistry::load_from_assets());
    commands.insert_resource(RecipeRegistry::load_from_assets());
}

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup);
    }
}