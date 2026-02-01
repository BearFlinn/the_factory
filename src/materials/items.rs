use bevy::prelude::*;
use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ItemName = String;

pub trait InventoryAccess {
    fn items(&self) -> &HashMap<ItemName, u32>;

    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32>;

    fn capacity(&self) -> u32;

    /// Does not enforce capacity limits.
    fn add_item(&mut self, item_name: &str, quantity: u32) -> u32 {
        *self.items_mut().entry(item_name.to_string()).or_insert(0) += quantity;
        quantity
    }

    fn remove_item(&mut self, item_name: &str, quantity: u32) -> u32 {
        if let Some(current_quantity) = self.items_mut().get_mut(item_name) {
            let removed = (*current_quantity).min(quantity);
            *current_quantity -= removed;
            if *current_quantity == 0 {
                self.items_mut().remove(item_name);
            }
            removed
        } else {
            0
        }
    }

    fn get_item_quantity(&self, item_name: &str) -> u32 {
        self.items().get(item_name).copied().unwrap_or(0)
    }

    fn get_total_quantity(&self) -> u32 {
        self.items().values().sum::<u32>()
    }

    fn is_full(&self) -> bool {
        self.get_total_quantity() >= self.capacity()
    }

    fn is_empty(&self) -> bool {
        self.items().is_empty()
    }

    fn has_space_for(&self, items: &HashMap<ItemName, u32>) -> bool {
        let current_quantity = self.get_total_quantity();
        let total_quantity = items.values().sum::<u32>();
        current_quantity + total_quantity <= self.capacity()
    }

    fn has_at_least(&self, item_name: &str, required_quantity: u32) -> bool {
        self.get_item_quantity(item_name) >= required_quantity
    }

    fn has_items_for_recipe(&self, recipe: &HashMap<ItemName, u32>) -> bool {
        recipe
            .iter()
            .all(|(item_name, quantity)| self.has_at_least(item_name, *quantity))
    }

    fn get_all_items(&self) -> HashMap<ItemName, u32> {
        self.items().clone()
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct OutputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl OutputPort {
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }
}

impl InventoryAccess for OutputPort {
    fn items(&self) -> &HashMap<ItemName, u32> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32> {
        &mut self.items
    }

    fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct InputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl InputPort {
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }
}

impl InventoryAccess for InputPort {
    fn items(&self) -> &HashMap<ItemName, u32> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32> {
        &mut self.items
    }

    fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct StoragePort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl StoragePort {
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }
}

impl InventoryAccess for StoragePort {
    fn items(&self) -> &HashMap<ItemName, u32> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32> {
        &mut self.items
    }

    fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Cargo {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl Cargo {
    #[must_use]
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }
}

impl InventoryAccess for Cargo {
    fn items(&self) -> &HashMap<ItemName, u32> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32> {
        &mut self.items
    }

    fn capacity(&self) -> u32 {
        self.capacity
    }
}

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

    /// Load item definitions from embedded assets.
    ///
    /// # Errors
    /// Returns an error if the embedded RON content fails to parse.
    pub fn load_from_assets() -> Result<Self, Box<dyn std::error::Error>> {
        let ron_content = include_str!("../assets/items.ron");
        Self::from_ron(ron_content)
    }

    pub fn get_definition(&self, item_name: &str) -> Option<&ItemDef> {
        self.definitions.get(item_name)
    }
}

#[derive(Debug)]
pub enum TransferError {
    ItemNotFound,
    NotEnoughItems,
    DestinationFull,
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ItemNotFound => write!(f, "Item not found!"),
            Self::NotEnoughItems => write!(f, "Not enough items to transfer!"),
            Self::DestinationFull => write!(f, "Destination storage full!"),
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
    output_ports: Query<&OutputPort>,
    input_ports: Query<&InputPort>,
    storage_ports: Query<&StoragePort>,
    cargo_query: Query<&Cargo>,
) {
    for request in requests.read() {
        let sender_data =
            get_sender_port_data(request.sender, &output_ports, &storage_ports, &cargo_query);

        let Some((sender_items, _sender_capacity)) = sender_data else {
            validation_events.write(ItemTransferValidationEvent {
                result: Err(TransferError::ItemNotFound),
                request: request.clone(),
            });
            continue;
        };

        let receiver_data =
            get_receiver_port_data(request.receiver, &input_ports, &storage_ports, &cargo_query);

        let Some((receiver_total, receiver_capacity)) = receiver_data else {
            validation_events.write(ItemTransferValidationEvent {
                result: Err(TransferError::ItemNotFound),
                request: request.clone(),
            });
            continue;
        };

        let mut validated_transfer = HashMap::new();
        let mut current_receiver_total = receiver_total;

        for (item_name, &requested_quantity) in &request.items {
            let available = sender_items.get(item_name).copied().unwrap_or(0);

            if available == 0 {
                continue;
            }

            let transfer_quantity = available.min(requested_quantity);
            let remaining_capacity = receiver_capacity.saturating_sub(current_receiver_total);

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
            let error = if request
                .items
                .iter()
                .all(|(name, _)| sender_items.get(name).copied().unwrap_or(0) == 0)
            {
                TransferError::NotEnoughItems
            } else {
                TransferError::DestinationFull
            };

            validation_events.write(ItemTransferValidationEvent {
                result: Err(error),
                request: request.clone(),
            });
        } else {
            validation_events.write(ItemTransferValidationEvent {
                result: Ok(validated_transfer),
                request: request.clone(),
            });
        }
    }
}

fn get_sender_port_data(
    entity: Entity,
    output_ports: &Query<&OutputPort>,
    storage_ports: &Query<&StoragePort>,
    cargo_query: &Query<&Cargo>,
) -> Option<(HashMap<ItemName, u32>, u32)> {
    if let Ok(port) = output_ports.get(entity) {
        return Some((port.items.clone(), port.capacity));
    }
    if let Ok(port) = storage_ports.get(entity) {
        return Some((port.items.clone(), port.capacity));
    }
    if let Ok(cargo) = cargo_query.get(entity) {
        return Some((cargo.items.clone(), cargo.capacity));
    }
    None
}

fn get_receiver_port_data(
    entity: Entity,
    input_ports: &Query<&InputPort>,
    storage_ports: &Query<&StoragePort>,
    cargo_query: &Query<&Cargo>,
) -> Option<(u32, u32)> {
    if let Ok(port) = input_ports.get(entity) {
        let total: u32 = port.items.values().sum();
        return Some((total, port.capacity));
    }
    if let Ok(port) = storage_ports.get(entity) {
        let total: u32 = port.items.values().sum();
        return Some((total, port.capacity));
    }
    if let Ok(cargo) = cargo_query.get(entity) {
        let total: u32 = cargo.items.values().sum();
        return Some((total, cargo.capacity));
    }
    None
}

pub fn execute_item_transfer(
    mut validation_events: EventReader<ItemTransferValidationEvent>,
    mut output_ports: Query<&mut OutputPort>,
    mut input_ports: Query<&mut InputPort>,
    mut storage_ports: Query<&mut StoragePort>,
    mut cargo_query: Query<&mut Cargo>,
    mut transfer_events: EventWriter<ItemTransferEvent>,
) {
    for validation in validation_events.read() {
        let Ok(validated_items) = &validation.result else {
            continue;
        };

        if validated_items.is_empty() {
            continue;
        }

        let sender = validation.request.sender;
        let receiver = validation.request.receiver;

        if sender == receiver {
            continue;
        }

        let mut actual_transfer = HashMap::new();

        if let Ok(mut port) = output_ports.get_mut(sender) {
            for (item_name, &quantity) in validated_items {
                let removed = port.remove_item(item_name, quantity);
                if removed > 0 {
                    actual_transfer.insert(item_name.clone(), removed);
                }
            }
        } else if let Ok(mut port) = storage_ports.get_mut(sender) {
            for (item_name, &quantity) in validated_items {
                let removed = port.remove_item(item_name, quantity);
                if removed > 0 {
                    actual_transfer.insert(item_name.clone(), removed);
                }
            }
        } else if let Ok(mut cargo) = cargo_query.get_mut(sender) {
            for (item_name, &quantity) in validated_items {
                let removed = cargo.remove_item(item_name, quantity);
                if removed > 0 {
                    actual_transfer.insert(item_name.clone(), removed);
                }
            }
        }

        if actual_transfer.is_empty() {
            continue;
        }

        if let Ok(mut port) = input_ports.get_mut(receiver) {
            for (item_name, &quantity) in &actual_transfer {
                port.add_item(item_name, quantity);
            }
        } else if let Ok(mut port) = storage_ports.get_mut(receiver) {
            for (item_name, &quantity) in &actual_transfer {
                port.add_item(item_name, quantity);
            }
        } else if let Ok(mut cargo) = cargo_query.get_mut(receiver) {
            for (item_name, &quantity) in &actual_transfer {
                cargo.add_item(item_name, quantity);
            }
        }

        transfer_events.write(ItemTransferEvent {
            sender,
            receiver,
            items_transferred: actual_transfer,
        });
    }
}

pub fn request_transfer_specific_items(
    sender: Entity,
    receiver: Entity,
    items: HashMap<ItemName, u32>,
    transfer_events: &mut EventWriter<ItemTransferRequestEvent>,
) {
    if !items.is_empty() {
        transfer_events.write(ItemTransferRequestEvent {
            sender,
            receiver,
            items,
        });
    }
}

fn setup(mut commands: Commands) {
    if let Ok(registry) = ItemRegistry::load_from_assets() {
        commands.insert_resource(registry);
    }
}

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn test_item_registry_from_ron_valid() {
        let ron_content = r#"[
            (
                name: "Test Item",
                tier: 1,
            ),
            (
                name: "Another Item",
                tier: 2,
            ),
        ]"#;
        let registry = ItemRegistry::from_ron(ron_content).unwrap();
        assert_eq!(registry.definitions.len(), 2);
        assert!(registry.definitions.contains_key("Test Item"));
        assert!(registry.definitions.contains_key("Another Item"));
    }

    #[test]
    fn test_item_registry_from_ron_empty() {
        let ron_content = "[]";
        let registry = ItemRegistry::from_ron(ron_content).unwrap();
        assert!(registry.definitions.is_empty());
    }

    #[test]
    fn test_item_registry_from_ron_invalid() {
        let ron_content = "invalid ron content";
        let result = ItemRegistry::from_ron(ron_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_item_registry_from_ron_missing_field() {
        let ron_content = r#"[
            (
                name: "Test Item",
            ),
        ]"#;
        let result = ItemRegistry::from_ron(ron_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_item_registry_get_definition_existing() {
        let ron_content = r#"[
            (
                name: "Test Item",
                tier: 3,
            ),
        ]"#;
        let registry = ItemRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Test Item");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.name, "Test Item");
        assert_eq!(def.tier, 3);
    }

    #[test]
    fn test_item_registry_get_definition_nonexistent() {
        let ron_content = "[]";
        let registry = ItemRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Nonexistent");
        assert!(def.is_none());
    }

    #[test]
    fn test_transfer_error_display_item_not_found() {
        let error = TransferError::ItemNotFound;
        assert_eq!(format!("{error}"), "Item not found!");
    }

    #[test]
    fn test_transfer_error_display_not_enough_items() {
        let error = TransferError::NotEnoughItems;
        assert_eq!(format!("{error}"), "Not enough items to transfer!");
    }

    #[test]
    fn test_transfer_error_display_destination_full() {
        let error = TransferError::DestinationFull;
        assert_eq!(format!("{error}"), "Destination storage full!");
    }

    #[test]
    fn test_is_full_under_capacity() {
        let mut storage = StoragePort::new(100);
        storage.add_item("iron", 50);
        assert!(!storage.is_full());
    }

    #[test]
    fn test_is_full_at_capacity() {
        let mut storage = StoragePort::new(100);
        storage.add_item("iron", 100);
        assert!(storage.is_full());
    }

    #[test]
    fn test_is_full_over_capacity() {
        let mut storage = StoragePort::new(100);
        storage.add_item("iron", 101);
        assert!(storage.is_full());
    }
}
