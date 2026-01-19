use bevy::prelude::{App, Commands, IntoSystemConfigs, Plugin, Startup, Update};

pub mod items;
pub mod recipes;

pub use items::{
    execute_item_transfer, request_transfer_specific_items, validate_item_transfer, Cargo,
    InputPort, Inventory, InventoryAccess, ItemName, ItemRegistry, ItemTransferEvent,
    ItemTransferRequestEvent, ItemTransferValidationEvent, OutputPort, StoragePort,
};
pub use recipes::{RecipeDef, RecipeName, RecipeRegistry};

pub struct MaterialsPlugin;

fn setup(mut commands: Commands) {
    if let Ok(registry) = ItemRegistry::load_from_assets() {
        commands.insert_resource(registry);
    }
    if let Ok(registry) = RecipeRegistry::load_from_assets() {
        commands.insert_resource(registry);
    }
}

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ItemTransferRequestEvent>()
            .add_event::<ItemTransferValidationEvent>()
            .add_event::<ItemTransferEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    validate_item_transfer,
                    execute_item_transfer,
                    // print_transferred_items
                )
                    .chain(),
            );
    }
}
