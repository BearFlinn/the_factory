use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::scene::ron;

pub type ItemName = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemDef {
    pub name: String,
    pub tier: u32,
    // pub stack_size: u32 (not needed yet)
}

#[derive(Resource)]
pub struct ItemRegistry {
    pub definitions: HashMap<ItemName, ItemDef>,
}

impl ItemRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<ItemDef> = ron::from_str(ron_content)?;
        
        let mut definitions = HashMap::new();
        
        for def in definitions_vec {
            definitions.insert(def.name.clone(), def);
        }
        
        Ok(Self { definitions })
    }

    pub fn load_from_assets() -> Self {
        let ron_content = include_str!("../assets/items.ron");
        Self::from_ron(ron_content).expect("Failed to load item definitions")
    }

    pub fn get_definition(&self, item_name: &str) -> Option<&ItemDef> {
        self.definitions.get(item_name)
    }
    // TODO: Add methods for accessing individual item fields from definitions
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InventoryTypes {
    #[default]
    Storage,
    Sender,
    Requester,
    Carrier,
    Producer,
}

#[derive(Component, Default, Serialize, Deserialize, Debug, Clone)]
pub struct InventoryType(pub InventoryTypes);


#[derive(Component)]
#[require(InventoryType)]
pub struct Inventory {
    pub items: HashMap<ItemName, u32>, 
    pub capacity: u32,
}

impl Inventory {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }

    pub fn add_item(&mut self, item_name: &str, quantity: u32) -> u32 {
        *self.items.entry(item_name.to_string()).or_insert(0) += quantity;
        quantity
    }

    pub fn remove_item(&mut self, item_name: &str, quantity: u32) -> u32 {
        if let Some(current_quantity) = self.items.get_mut(item_name) {
            let removed = (*current_quantity).min(quantity);
            *current_quantity -= removed;
            if *current_quantity == 0 {
                self.items.remove(item_name);
            }
            removed
        } else {
            0
        }
    }

    pub fn has_recipe_outputs(&self, recipe: &HashMap<ItemName, u32>) -> HashMap<ItemName, u32> {
        let mut outputs = HashMap::new();
        for (item_name, quantity) in recipe {
            if self.has_at_least(item_name, *quantity) {
                outputs.insert(item_name.clone(), *quantity);
            }
        }
        outputs
    }

    pub fn has_items_for_recipe(&self, recipe: &HashMap<ItemName, u32>) -> bool {
        recipe.iter().all(|(item_name, quantity)| self.has_at_least(item_name, *quantity))
    }

    pub fn remove_items_for_recipe(&mut self, recipe: &HashMap<ItemName, u32>) -> HashMap<ItemName, u32> {
        let mut removed = HashMap::new();
        for (item_name, quantity) in recipe {
            let removed_quantity = self.remove_item(item_name, *quantity);
            removed.insert(item_name.clone(), removed_quantity);
        }
        removed
    }

    pub fn is_full(&self) -> bool {
        let current_quantity = self.items.values().sum::<u32>();
        current_quantity == self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_space_for(&self, items: &HashMap<ItemName, u32>) -> bool {
        let current_quantity = self.items.values().sum::<u32>();
        let total_quantity = items.values().sum::<u32>();
        current_quantity + total_quantity <= self.capacity
    }

    pub fn get_all_items(&self) -> HashMap<ItemName, u32> {
        self.items.clone()
    }

    pub fn get_item_quantity(&self, item_name: &str) -> u32 {
        self.items.get(item_name).copied().unwrap_or(0)
    }

    pub fn get_total_quantity(&self) -> u32 {
        self.items.values().sum::<u32>()
    }

    pub fn has_item(&self, item_name: &str) -> bool {
        self.get_item_quantity(item_name) > 0
    }

    pub fn has_any_item(&self) -> bool {
        self.items.values().sum::<u32>() > 0
    }

    pub fn has_at_least(&self, item_name: &str, required_quantity: u32) -> bool {
        self.get_item_quantity(item_name) >= required_quantity
    }

    pub fn has_less_than(&self, item_name: &str, required_quantity: u32) -> bool {
        self.get_item_quantity(item_name) < required_quantity
    }
}

#[derive(Debug)]
pub enum TransferError {
    ItemNotFound,
    NotEnoughItems,
    InventoryFull,
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::ItemNotFound => write!(f, "Item not found!"),
            TransferError::NotEnoughItems => write!(f, "Not enough items to transfer!"),
            TransferError::InventoryFull => write!(f, "Inventory full!"),
            _ => write!(f, "Unknown transfer error!"),
        }
    }
}

#[derive(Event, Clone)]
pub struct ItemTransferRequestEvent {
    pub sender: Entity,
    pub receiver: Entity,
    pub items: HashMap<ItemName, u32>,
}

#[derive(Event)]
pub struct ItemTransferValidationEvent {
    pub result: Result<HashMap<ItemName, u32>, TransferError>,
    pub request: ItemTransferRequestEvent,
}

#[derive(Event, Debug)]
#[allow(dead_code)]
pub struct ItemTransferEvent {
    pub sender: Entity,
    pub receiver: Entity,
    pub items_transferred: HashMap<ItemName, u32>,
}

#[allow(dead_code)]
pub fn print_transferred_items(mut events: EventReader<ItemTransferEvent>) {
    for event in events.read() {
        println!("Items transferred: {:?}", event.items_transferred);
    }
}

pub fn validate_item_transfer(
    mut requests: EventReader<ItemTransferRequestEvent>,
    mut validation_events: EventWriter<ItemTransferValidationEvent>,
    inventories: Query<&Inventory>,
) {
    for request in requests.read() {
        let sender_inventory = match inventories.get(request.sender) {
            Ok(inv) => inv,
            Err(_) => {
                validation_events.send(ItemTransferValidationEvent {
                    result: Err(TransferError::ItemNotFound),
                    request: request.clone(),
                });
                continue;
            }
        };

        let receiver_inventory = match inventories.get(request.receiver) {
            Ok(inv) => inv,
            Err(_) => {
                validation_events.send(ItemTransferValidationEvent {
                    result: Err(TransferError::ItemNotFound),
                    request: request.clone(),
                });
                continue;
            }
        };

        let mut validated_transfer = HashMap::new();
        let mut current_receiver_total = receiver_inventory.items.values().sum::<u32>();

        for (item_name, &requested_quantity) in &request.items {
            let available = sender_inventory.get_item_quantity(item_name);
            
            if available == 0 {
                continue;
            }

            let transfer_quantity = available.min(requested_quantity);
            let remaining_capacity = receiver_inventory.capacity.saturating_sub(current_receiver_total);
            
            if remaining_capacity == 0 {
                break;
            }

            let final_quantity = transfer_quantity.min(remaining_capacity);
            
            if final_quantity > 0 {
                validated_transfer.insert(item_name.clone(), final_quantity);
                current_receiver_total += final_quantity;
            }
        }

        if validated_transfer.is_empty() {
            let error = if request.items.iter().all(|(name, _)| sender_inventory.get_item_quantity(name) == 0) {
                TransferError::NotEnoughItems
            } else {
                TransferError::InventoryFull
            };
            
            validation_events.send(ItemTransferValidationEvent {
                result: Err(error),
                request: request.clone(),
            });
        } else {
            validation_events.send(ItemTransferValidationEvent {
                result: Ok(validated_transfer),
                request: request.clone(),
            });
        }
    }
}

pub fn execute_item_transfer(
    mut validation_events: EventReader<ItemTransferValidationEvent>,
    mut inventories: Query<&mut Inventory>,
    mut transfer_events: EventWriter<ItemTransferEvent>,
) {
    for validation in validation_events.read() {
        if let Ok(validated_items) = &validation.result {
            if validated_items.is_empty() {
                continue;
            }

            let sender = validation.request.sender;
            let receiver = validation.request.receiver;
            
            if sender == receiver {
                continue;
            }

            let mut actual_transfer = HashMap::new();

            if let Ok(mut sender_inv) = inventories.get_mut(sender) {
                for (item_name, &quantity) in validated_items {
                    let removed = sender_inv.remove_item(item_name, quantity);
                    if removed > 0 {
                        actual_transfer.insert(item_name.clone(), removed);
                    }
                }
            }

            if !actual_transfer.is_empty() {
                if let Ok(mut receiver_inv) = inventories.get_mut(receiver) {
                    for (item_name, &quantity) in &actual_transfer {
                        receiver_inv.add_item(item_name, quantity);
                    }
                }

                transfer_events.send(ItemTransferEvent {
                    sender,
                    receiver,
                    items_transferred: actual_transfer,
                });
            }
        }
    }
}

pub fn request_transfer_all_items(
    sender: Entity,
    receiver: Entity,
    transfer_events: &mut EventWriter<ItemTransferRequestEvent>,
    inventories: &Query<&Inventory>,
) {
    if let Ok(sender_inventory) = inventories.get(sender) {
        let all_items = sender_inventory.get_all_items();
        if !all_items.is_empty() {
            transfer_events.send(ItemTransferRequestEvent {
                sender,
                receiver,
                items: all_items,
            });
        }
    }
}

pub fn request_transfer_specific_items(
    sender: Entity,
    receiver: Entity,
    items: HashMap<ItemName, u32>,
    transfer_events: &mut EventWriter<ItemTransferRequestEvent>,
) {
    if !items.is_empty() {
        transfer_events.send(ItemTransferRequestEvent {
            sender,
            receiver,
            items,
        });
    }
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(ItemRegistry::load_from_assets());
}
pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup);
    }
}
