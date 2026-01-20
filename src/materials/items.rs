use bevy::prelude::*;
use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ItemName = String;

// ============================================================================
// Inventory Access Trait
// ============================================================================

/// Shared behavior for all inventory-like components.
/// Enables uniform operations across different port types.
pub trait InventoryAccess {
    /// Returns immutable reference to the items map.
    fn items(&self) -> &HashMap<ItemName, u32>;

    /// Returns mutable reference to the items map.
    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32>;

    /// Returns the maximum capacity of this inventory.
    fn capacity(&self) -> u32;

    /// Adds items to the inventory. Returns the quantity added.
    /// Note: Does not enforce capacity limits.
    fn add_item(&mut self, item_name: &str, quantity: u32) -> u32 {
        *self.items_mut().entry(item_name.to_string()).or_insert(0) += quantity;
        quantity
    }

    /// Removes items from the inventory. Returns the quantity actually removed.
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

    /// Returns the quantity of a specific item in the inventory.
    fn get_item_quantity(&self, item_name: &str) -> u32 {
        self.items().get(item_name).copied().unwrap_or(0)
    }

    /// Returns the total quantity of all items in the inventory.
    fn get_total_quantity(&self) -> u32 {
        self.items().values().sum::<u32>()
    }

    /// Returns true if the inventory is at capacity.
    fn is_full(&self) -> bool {
        self.get_total_quantity() == self.capacity()
    }

    /// Returns true if the inventory contains no items.
    fn is_empty(&self) -> bool {
        self.items().is_empty()
    }

    /// Returns true if the inventory has space for the given items.
    fn has_space_for(&self, items: &HashMap<ItemName, u32>) -> bool {
        let current_quantity = self.get_total_quantity();
        let total_quantity = items.values().sum::<u32>();
        current_quantity + total_quantity <= self.capacity()
    }

    /// Returns true if the inventory has at least the specified quantity of an item.
    fn has_at_least(&self, item_name: &str, required_quantity: u32) -> bool {
        self.get_item_quantity(item_name) >= required_quantity
    }

    /// Returns true if the inventory has all items required for a recipe.
    fn has_items_for_recipe(&self, recipe: &HashMap<ItemName, u32>) -> bool {
        recipe
            .iter()
            .all(|(item_name, quantity)| self.has_at_least(item_name, *quantity))
    }

    /// Returns a clone of all items in the inventory.
    fn get_all_items(&self) -> HashMap<ItemName, u32> {
        self.items().clone()
    }
}

// ============================================================================
// Port Components
// ============================================================================

/// Items can be picked up from here (Mining Drills, Smelter outputs).
/// Used for buildings that provide items for logistics.
#[derive(Component, Default, Debug, Clone)]
pub struct OutputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl OutputPort {
    /// Creates a new output port with the specified capacity.
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

/// Items can be delivered here (Generators, Smelter inputs).
/// Used for buildings that accept items for processing.
#[derive(Component, Default, Debug, Clone)]
pub struct InputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl InputPort {
    /// Creates a new input port with the specified capacity.
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

/// Bidirectional storage - accepts deliveries and provides pickups.
/// Used for storage buildings that can both receive and provide items.
#[derive(Component, Default, Debug, Clone)]
pub struct StoragePort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl StoragePort {
    /// Creates a new storage port with the specified capacity.
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

/// Transient carrying capacity for workers.
/// Used for entities that transport items between buildings.
#[derive(Component, Default, Debug, Clone)]
pub struct Cargo {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl Cargo {
    /// Creates a new cargo component with the specified capacity.
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

// ============================================================================
// Item Registry
// ============================================================================

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
    // TODO: Add methods for accessing individual item fields from definitions
}

// ============================================================================
// TRANSFER ERRORS AND EVENTS
// ============================================================================

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

/// Validates item transfer requests using explicit port component queries.
/// No fallback chains - sender must have `OutputPort`, `StoragePort`, or `Cargo`.
/// Receiver must have `InputPort`, `StoragePort`, or `Cargo`.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn validate_item_transfer(
    mut requests: EventReader<ItemTransferRequestEvent>,
    mut validation_events: EventWriter<ItemTransferValidationEvent>,
    output_ports: Query<&OutputPort>,
    input_ports: Query<&InputPort>,
    storage_ports: Query<&StoragePort>,
    cargo_query: Query<&Cargo>,
) {
    for request in requests.read() {
        // Get sender's items and capacity using explicit port matching
        let sender_data =
            get_sender_port_data(request.sender, &output_ports, &storage_ports, &cargo_query);

        let Some((sender_items, _sender_capacity)) = sender_data else {
            validation_events.send(ItemTransferValidationEvent {
                result: Err(TransferError::ItemNotFound),
                request: request.clone(),
            });
            continue;
        };

        // Get receiver's capacity and current total using explicit port matching
        let receiver_data =
            get_receiver_port_data(request.receiver, &input_ports, &storage_ports, &cargo_query);

        let Some((receiver_total, receiver_capacity)) = receiver_data else {
            validation_events.send(ItemTransferValidationEvent {
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

/// Gets sender port data (items, capacity) from `OutputPort`, `StoragePort`, or `Cargo`.
/// Returns None if the entity doesn't have any valid sender port.
fn get_sender_port_data(
    entity: Entity,
    output_ports: &Query<&OutputPort>,
    storage_ports: &Query<&StoragePort>,
    cargo_query: &Query<&Cargo>,
) -> Option<(HashMap<ItemName, u32>, u32)> {
    // Check OutputPort first (for buildings that produce items)
    if let Ok(port) = output_ports.get(entity) {
        return Some((port.items.clone(), port.capacity));
    }
    // Check StoragePort (for storage buildings)
    if let Ok(port) = storage_ports.get(entity) {
        return Some((port.items.clone(), port.capacity));
    }
    // Check Cargo (for workers)
    if let Ok(cargo) = cargo_query.get(entity) {
        return Some((cargo.items.clone(), cargo.capacity));
    }
    None
}

/// Gets receiver port data (current total, capacity) from `InputPort`, `StoragePort`, or `Cargo`.
/// Returns None if the entity doesn't have any valid receiver port.
fn get_receiver_port_data(
    entity: Entity,
    input_ports: &Query<&InputPort>,
    storage_ports: &Query<&StoragePort>,
    cargo_query: &Query<&Cargo>,
) -> Option<(u32, u32)> {
    // Check InputPort first (for buildings that consume items)
    if let Ok(port) = input_ports.get(entity) {
        let total: u32 = port.items.values().sum();
        return Some((total, port.capacity));
    }
    // Check StoragePort (for storage buildings)
    if let Ok(port) = storage_ports.get(entity) {
        let total: u32 = port.items.values().sum();
        return Some((total, port.capacity));
    }
    // Check Cargo (for workers)
    if let Ok(cargo) = cargo_query.get(entity) {
        let total: u32 = cargo.items.values().sum();
        return Some((total, cargo.capacity));
    }
    None
}

/// Executes validated item transfers using explicit port component queries.
/// Removes from sender's `OutputPort`, `StoragePort`, or `Cargo`.
/// Adds to receiver's `InputPort`, `StoragePort`, or `Cargo`.
#[allow(clippy::type_complexity)]
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

        // Remove items from sender using explicit port matching
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

        // Add items to receiver using explicit port matching
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

        transfer_events.send(ItemTransferEvent {
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
        transfer_events.send(ItemTransferRequestEvent {
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

    // ==================== ItemRegistry::from_ron() tests ====================

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

    // ==================== TransferError Display tests ====================

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
}
