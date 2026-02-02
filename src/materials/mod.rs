use bevy::prelude::{error, App, IntoScheduleConfigs, Plugin, Update};

pub mod items;
pub mod recipes;

pub use items::{
    execute_item_transfer, request_transfer_specific_items, validate_item_transfer, Cargo,
    InputPort, InventoryAccess, ItemName, ItemRegistry, ItemTransferEvent,
    ItemTransferRequestEvent, ItemTransferValidationEvent, OutputPort, StoragePort,
};
pub use recipes::{RecipeDef, RecipeName, RecipeRegistry};

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        match ItemRegistry::load_from_assets() {
            Ok(registry) => {
                app.insert_resource(registry);
            }
            Err(e) => {
                error!("failed to load item registry: {e}");
            }
        }
        match RecipeRegistry::load_from_assets() {
            Ok(registry) => {
                app.insert_resource(registry);
            }
            Err(e) => {
                error!("failed to load recipe registry: {e}");
            }
        }

        app.add_message::<ItemTransferRequestEvent>()
            .add_message::<ItemTransferValidationEvent>()
            .add_message::<ItemTransferEvent>()
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
