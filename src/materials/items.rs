use bevy::prelude::*;
use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InventoryTypes {
    #[default]
    Storage,
    Sender,
    Requester,
    Carrier,
    // TODO: Implement a better system for buildings with inputs and outputs
    Producer,
}

#[derive(Component, Default, Serialize, Deserialize, Debug, Clone)]
pub struct InventoryType(pub InventoryTypes);

// TODO: Move this to its own module
#[derive(Component, Debug)]
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
        recipe
            .iter()
            .all(|(item_name, quantity)| self.has_at_least(item_name, *quantity))
    }

    pub fn remove_items_for_recipe(
        &mut self,
        recipe: &HashMap<ItemName, u32>,
    ) -> HashMap<ItemName, u32> {
        let mut removed = HashMap::new();
        for (item_name, quantity) in recipe {
            let removed_quantity = self.remove_item(item_name, *quantity);
            removed.insert(item_name.clone(), removed_quantity);
        }
        removed
    }

    pub fn recipe_output_amounts(&self, recipe: &HashMap<ItemName, u32>) -> HashMap<ItemName, u32> {
        let mut output_amounts = HashMap::new();
        for (item_name, quantity) in recipe {
            output_amounts.insert(
                item_name.clone(),
                self.get_item_quantity(item_name) * quantity,
            );
        }
        output_amounts
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

/// Input buffer for production buildings - holds items waiting to be processed.
/// Separating input from output prevents mixing of raw materials with finished products,
/// making logistics simpler and inventory jams easier to diagnose.
#[derive(Component, Debug)]
pub struct InputBuffer {
    /// The inventory that holds incoming items for processing.
    pub inventory: Inventory,
    /// Request more items when fill level drops below this percentage (0.0-1.0).
    pub request_threshold: f32,
}

impl InputBuffer {
    /// Creates a new input buffer with the specified capacity and request threshold.
    pub fn new(capacity: u32, request_threshold: f32) -> Self {
        Self {
            inventory: Inventory::new(capacity),
            request_threshold: request_threshold.clamp(0.0, 1.0),
        }
    }

    /// Returns the current fill level as a percentage (0.0-1.0).
    /// Returns 1.0 for zero-capacity buffers to prevent division by zero.
    #[allow(clippy::cast_precision_loss)]
    pub fn fill_level(&self) -> f32 {
        if self.inventory.capacity == 0 {
            return 1.0;
        }
        self.inventory.get_total_quantity() as f32 / self.inventory.capacity as f32
    }

    /// Returns true if the buffer needs more items (fill level below request threshold).
    pub fn needs_items(&self) -> bool {
        self.fill_level() < self.request_threshold
    }
}

/// Output buffer for production buildings - holds items that have been produced.
/// Separating output from input ensures finished products don't compete with raw
/// materials for inventory space.
#[derive(Component, Debug)]
pub struct OutputBuffer {
    /// The inventory that holds produced items awaiting pickup.
    pub inventory: Inventory,
    /// Offer items for pickup when fill level exceeds this percentage (0.0-1.0).
    pub offer_threshold: f32,
}

impl OutputBuffer {
    /// Creates a new output buffer with the specified capacity and offer threshold.
    pub fn new(capacity: u32, offer_threshold: f32) -> Self {
        Self {
            inventory: Inventory::new(capacity),
            offer_threshold: offer_threshold.clamp(0.0, 1.0),
        }
    }

    /// Returns the current fill level as a percentage (0.0-1.0).
    /// Returns 1.0 for zero-capacity buffers to prevent division by zero.
    #[allow(clippy::cast_precision_loss)]
    pub fn fill_level(&self) -> f32 {
        if self.inventory.capacity == 0 {
            return 1.0;
        }
        self.inventory.get_total_quantity() as f32 / self.inventory.capacity as f32
    }

    /// Returns true if the buffer has items ready to offer (fill level above offer threshold).
    pub fn has_items_to_offer(&self) -> bool {
        self.fill_level() > self.offer_threshold
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
            Self::ItemNotFound => write!(f, "Item not found!"),
            Self::NotEnoughItems => write!(f, "Not enough items to transfer!"),
            Self::InventoryFull => write!(f, "Inventory full!"),
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

#[allow(clippy::needless_pass_by_value)] // Bevy system parameter
pub fn validate_item_transfer(
    mut requests: EventReader<ItemTransferRequestEvent>,
    mut validation_events: EventWriter<ItemTransferValidationEvent>,
    inventories: Query<&Inventory>,
) {
    for request in requests.read() {
        let Ok(sender_inventory) = inventories.get(request.sender) else {
            validation_events.send(ItemTransferValidationEvent {
                result: Err(TransferError::ItemNotFound),
                request: request.clone(),
            });
            continue;
        };

        let Ok(receiver_inventory) = inventories.get(request.receiver) else {
            validation_events.send(ItemTransferValidationEvent {
                result: Err(TransferError::ItemNotFound),
                request: request.clone(),
            });
            continue;
        };

        let mut validated_transfer = HashMap::new();
        let mut current_receiver_total = receiver_inventory.items.values().sum::<u32>();

        for (item_name, &requested_quantity) in &request.items {
            let available = sender_inventory.get_item_quantity(item_name);

            if available == 0 {
                continue;
            }

            let transfer_quantity = available.min(requested_quantity);
            let remaining_capacity = receiver_inventory
                .capacity
                .saturating_sub(current_receiver_total);

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
                .all(|(name, _)| sender_inventory.get_item_quantity(name) == 0)
            {
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

    // ==================== Inventory::new() tests ====================

    #[test]
    fn test_new_creates_empty_inventory() {
        let inventory = Inventory::new(100);
        assert!(inventory.items.is_empty());
        assert_eq!(inventory.capacity, 100);
    }

    #[test]
    fn test_new_with_zero_capacity() {
        let inventory = Inventory::new(0);
        assert!(inventory.items.is_empty());
        assert_eq!(inventory.capacity, 0);
    }

    // ==================== add_item() tests ====================

    #[test]
    fn test_add_item_single_item() {
        let mut inventory = Inventory::new(100);
        let added = inventory.add_item("Iron Ore", 5);
        assert_eq!(added, 5);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 5);
    }

    #[test]
    fn test_add_item_multiple_different_items() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        inventory.add_item("Copper Ore", 10);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 5);
        assert_eq!(inventory.get_item_quantity("Copper Ore"), 10);
    }

    #[test]
    fn test_add_item_same_item_twice_accumulates() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        inventory.add_item("Iron Ore", 3);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 8);
    }

    #[test]
    fn test_add_item_returns_quantity_added() {
        let mut inventory = Inventory::new(100);
        let added = inventory.add_item("Coal", 25);
        assert_eq!(added, 25);
    }

    #[test]
    fn test_add_item_does_not_cap_at_capacity() {
        // Note: The current implementation does NOT cap at capacity
        // This tests the actual behavior
        let mut inventory = Inventory::new(10);
        inventory.add_item("Iron Ore", 15);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 15);
    }

    // ==================== remove_item() tests ====================

    #[test]
    fn test_remove_item_basic() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        let removed = inventory.remove_item("Iron Ore", 5);
        assert_eq!(removed, 5);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 5);
    }

    #[test]
    fn test_remove_item_caps_at_available() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        let removed = inventory.remove_item("Iron Ore", 10);
        assert_eq!(removed, 5);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 0);
    }

    #[test]
    fn test_remove_item_nonexistent_returns_zero() {
        let mut inventory = Inventory::new(100);
        let removed = inventory.remove_item("Iron Ore", 5);
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_remove_item_removes_entry_when_zero() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        inventory.remove_item("Iron Ore", 5);
        assert!(!inventory.items.contains_key("Iron Ore"));
    }

    #[test]
    fn test_remove_item_partial_removal() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Copper Ore", 20);
        inventory.remove_item("Copper Ore", 7);
        assert_eq!(inventory.get_item_quantity("Copper Ore"), 13);
    }

    // ==================== is_full() tests ====================

    #[test]
    fn test_is_full_returns_true_when_at_capacity() {
        let mut inventory = Inventory::new(10);
        inventory.add_item("Iron Ore", 10);
        assert!(inventory.is_full());
    }

    #[test]
    fn test_is_full_returns_false_when_under_capacity() {
        let mut inventory = Inventory::new(10);
        inventory.add_item("Iron Ore", 5);
        assert!(!inventory.is_full());
    }

    #[test]
    fn test_is_full_empty_inventory() {
        let inventory = Inventory::new(10);
        assert!(!inventory.is_full());
    }

    #[test]
    fn test_is_full_zero_capacity_empty() {
        let inventory = Inventory::new(0);
        assert!(inventory.is_full());
    }

    // ==================== is_empty() tests ====================

    #[test]
    fn test_is_empty_returns_true_for_new_inventory() {
        let inventory = Inventory::new(100);
        assert!(inventory.is_empty());
    }

    #[test]
    fn test_is_empty_returns_false_with_items() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 1);
        assert!(!inventory.is_empty());
    }

    #[test]
    fn test_is_empty_returns_true_after_removing_all() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        inventory.remove_item("Iron Ore", 5);
        assert!(inventory.is_empty());
    }

    // ==================== has_space_for() tests ====================

    #[test]
    fn test_has_space_for_empty_inventory() {
        let inventory = Inventory::new(100);
        let items = HashMap::from([("Iron Ore".to_string(), 50u32)]);
        assert!(inventory.has_space_for(&items));
    }

    #[test]
    fn test_has_space_for_partial_inventory() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Copper Ore", 40);
        let items = HashMap::from([("Iron Ore".to_string(), 60u32)]);
        assert!(inventory.has_space_for(&items));
    }

    #[test]
    fn test_has_space_for_exactly_fits() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Copper Ore", 50);
        let items = HashMap::from([("Iron Ore".to_string(), 50u32)]);
        assert!(inventory.has_space_for(&items));
    }

    #[test]
    fn test_has_space_for_exceeds_capacity() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Copper Ore", 50);
        let items = HashMap::from([("Iron Ore".to_string(), 51u32)]);
        assert!(!inventory.has_space_for(&items));
    }

    #[test]
    fn test_has_space_for_empty_items() {
        let inventory = Inventory::new(100);
        let items = HashMap::new();
        assert!(inventory.has_space_for(&items));
    }

    // ==================== get_item_quantity() tests ====================

    #[test]
    fn test_get_item_quantity_existing_item() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 15);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 15);
    }

    #[test]
    fn test_get_item_quantity_nonexistent_item() {
        let inventory = Inventory::new(100);
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 0);
    }

    // ==================== get_total_quantity() tests ====================

    #[test]
    fn test_get_total_quantity_empty() {
        let inventory = Inventory::new(100);
        assert_eq!(inventory.get_total_quantity(), 0);
    }

    #[test]
    fn test_get_total_quantity_single_item() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        assert_eq!(inventory.get_total_quantity(), 10);
    }

    #[test]
    fn test_get_total_quantity_multiple_items() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        inventory.add_item("Copper Ore", 20);
        inventory.add_item("Coal", 5);
        assert_eq!(inventory.get_total_quantity(), 35);
    }

    // ==================== has_item() tests ====================

    #[test]
    fn test_has_item_existing() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        assert!(inventory.has_item("Iron Ore"));
    }

    #[test]
    fn test_has_item_nonexistent() {
        let inventory = Inventory::new(100);
        assert!(!inventory.has_item("Iron Ore"));
    }

    // ==================== has_at_least() tests ====================

    #[test]
    fn test_has_at_least_sufficient() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        assert!(inventory.has_at_least("Iron Ore", 5));
    }

    #[test]
    fn test_has_at_least_exact() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        assert!(inventory.has_at_least("Iron Ore", 10));
    }

    #[test]
    fn test_has_at_least_insufficient() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        assert!(!inventory.has_at_least("Iron Ore", 10));
    }

    #[test]
    fn test_has_at_least_nonexistent() {
        let inventory = Inventory::new(100);
        assert!(!inventory.has_at_least("Iron Ore", 1));
    }

    #[test]
    fn test_has_at_least_zero_required() {
        let inventory = Inventory::new(100);
        assert!(inventory.has_at_least("Iron Ore", 0));
    }

    // ==================== has_less_than() tests ====================

    #[test]
    fn test_has_less_than_true() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 5);
        assert!(inventory.has_less_than("Iron Ore", 10));
    }

    #[test]
    fn test_has_less_than_exact_false() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        assert!(!inventory.has_less_than("Iron Ore", 10));
    }

    #[test]
    fn test_has_less_than_more_false() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 15);
        assert!(!inventory.has_less_than("Iron Ore", 10));
    }

    #[test]
    fn test_has_less_than_nonexistent() {
        let inventory = Inventory::new(100);
        assert!(inventory.has_less_than("Iron Ore", 1));
    }

    // ==================== has_items_for_recipe() tests ====================

    #[test]
    fn test_has_items_for_recipe_sufficient() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        inventory.add_item("Coal", 5);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32), ("Coal".to_string(), 2u32)]);
        assert!(inventory.has_items_for_recipe(&recipe));
    }

    #[test]
    fn test_has_items_for_recipe_insufficient() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        inventory.add_item("Coal", 1);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32), ("Coal".to_string(), 2u32)]);
        assert!(!inventory.has_items_for_recipe(&recipe));
    }

    #[test]
    fn test_has_items_for_recipe_missing_item() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32), ("Coal".to_string(), 2u32)]);
        assert!(!inventory.has_items_for_recipe(&recipe));
    }

    #[test]
    fn test_has_items_for_recipe_empty_recipe() {
        let inventory = Inventory::new(100);
        let recipe = HashMap::new();
        assert!(inventory.has_items_for_recipe(&recipe));
    }

    // ==================== remove_items_for_recipe() tests ====================

    #[test]
    fn test_remove_items_for_recipe_success() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 10);
        inventory.add_item("Coal", 5);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32), ("Coal".to_string(), 2u32)]);
        let removed = inventory.remove_items_for_recipe(&recipe);
        assert_eq!(removed.get("Iron Ore"), Some(&5));
        assert_eq!(removed.get("Coal"), Some(&2));
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 5);
        assert_eq!(inventory.get_item_quantity("Coal"), 3);
    }

    #[test]
    fn test_remove_items_for_recipe_partial() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ore", 3);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32)]);
        let removed = inventory.remove_items_for_recipe(&recipe);
        assert_eq!(removed.get("Iron Ore"), Some(&3));
        assert_eq!(inventory.get_item_quantity("Iron Ore"), 0);
    }

    #[test]
    fn test_remove_items_for_recipe_missing_item() {
        let mut inventory = Inventory::new(100);
        let recipe = HashMap::from([("Iron Ore".to_string(), 5u32)]);
        let removed = inventory.remove_items_for_recipe(&recipe);
        assert_eq!(removed.get("Iron Ore"), Some(&0));
    }

    // ==================== has_recipe_outputs() tests ====================

    #[test]
    fn test_has_recipe_outputs_all_present() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Gear", 10);
        inventory.add_item("Iron Plate", 5);
        let recipe = HashMap::from([("Gear".to_string(), 2u32), ("Iron Plate".to_string(), 1u32)]);
        let outputs = inventory.has_recipe_outputs(&recipe);
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs.get("Gear"), Some(&2));
        assert_eq!(outputs.get("Iron Plate"), Some(&1));
    }

    #[test]
    fn test_has_recipe_outputs_partial() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Gear", 10);
        let recipe = HashMap::from([("Gear".to_string(), 2u32), ("Iron Plate".to_string(), 1u32)]);
        let outputs = inventory.has_recipe_outputs(&recipe);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs.get("Gear"), Some(&2));
        assert!(!outputs.contains_key("Iron Plate"));
    }

    #[test]
    fn test_has_recipe_outputs_none_present() {
        let inventory = Inventory::new(100);
        let recipe = HashMap::from([("Gear".to_string(), 2u32), ("Iron Plate".to_string(), 1u32)]);
        let outputs = inventory.has_recipe_outputs(&recipe);
        assert!(outputs.is_empty());
    }

    // ==================== recipe_output_amounts() tests ====================

    #[test]
    fn test_recipe_output_amounts_basic() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Iron Ingot", 5);
        let recipe = HashMap::from([("Gear".to_string(), 2u32)]);
        let outputs = inventory.recipe_output_amounts(&recipe);
        // get_item_quantity("Gear") = 0, 0 * 2 = 0
        assert_eq!(outputs.get("Gear"), Some(&0));
    }

    #[test]
    fn test_recipe_output_amounts_with_items() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Gear", 3);
        let recipe = HashMap::from([("Gear".to_string(), 2u32)]);
        let outputs = inventory.recipe_output_amounts(&recipe);
        // get_item_quantity("Gear") = 3, 3 * 2 = 6
        assert_eq!(outputs.get("Gear"), Some(&6));
    }

    #[test]
    fn test_recipe_output_amounts_multiple_items() {
        let mut inventory = Inventory::new(100);
        inventory.add_item("Gear", 3);
        inventory.add_item("Iron Plate", 2);
        let recipe = HashMap::from([("Gear".to_string(), 2u32), ("Iron Plate".to_string(), 5u32)]);
        let outputs = inventory.recipe_output_amounts(&recipe);
        assert_eq!(outputs.get("Gear"), Some(&6));
        assert_eq!(outputs.get("Iron Plate"), Some(&10));
    }

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
    fn test_transfer_error_display_inventory_full() {
        let error = TransferError::InventoryFull;
        assert_eq!(format!("{error}"), "Inventory full!");
    }

    // ==================== InputBuffer tests ====================

    #[test]
    fn test_input_buffer_new_creates_empty_buffer() {
        let buffer = InputBuffer::new(100, 0.3);
        assert!(buffer.inventory.is_empty());
        assert_eq!(buffer.inventory.capacity, 100);
        assert!((buffer.request_threshold - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_new_clamps_threshold_above_one() {
        let buffer = InputBuffer::new(100, 1.5);
        assert!((buffer.request_threshold - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_new_clamps_threshold_below_zero() {
        let buffer = InputBuffer::new(100, -0.5);
        assert!((buffer.request_threshold - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_fill_level_empty() {
        let buffer = InputBuffer::new(100, 0.3);
        assert!((buffer.fill_level() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_fill_level_partial() {
        let mut buffer = InputBuffer::new(100, 0.3);
        buffer.inventory.add_item("Iron Ore", 50);
        assert!((buffer.fill_level() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_fill_level_full() {
        let mut buffer = InputBuffer::new(100, 0.3);
        buffer.inventory.add_item("Iron Ore", 100);
        assert!((buffer.fill_level() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_fill_level_zero_capacity() {
        let buffer = InputBuffer::new(0, 0.3);
        // Zero-capacity buffer should report as full to avoid division by zero
        assert!((buffer.fill_level() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_input_buffer_needs_items_below_threshold() {
        let mut buffer = InputBuffer::new(100, 0.5);
        buffer.inventory.add_item("Iron Ore", 20); // 20% full, threshold 50%
        assert!(buffer.needs_items());
    }

    #[test]
    fn test_input_buffer_needs_items_at_threshold() {
        let mut buffer = InputBuffer::new(100, 0.5);
        buffer.inventory.add_item("Iron Ore", 50); // 50% full, threshold 50%
        assert!(!buffer.needs_items());
    }

    #[test]
    fn test_input_buffer_needs_items_above_threshold() {
        let mut buffer = InputBuffer::new(100, 0.5);
        buffer.inventory.add_item("Iron Ore", 80); // 80% full, threshold 50%
        assert!(!buffer.needs_items());
    }

    #[test]
    fn test_input_buffer_needs_items_zero_threshold() {
        let buffer = InputBuffer::new(100, 0.0);
        // With 0.0 threshold, never needs items (0.0 is not < 0.0)
        assert!(!buffer.needs_items());
    }

    // ==================== OutputBuffer tests ====================

    #[test]
    fn test_output_buffer_new_creates_empty_buffer() {
        let buffer = OutputBuffer::new(100, 0.2);
        assert!(buffer.inventory.is_empty());
        assert_eq!(buffer.inventory.capacity, 100);
        assert!((buffer.offer_threshold - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_new_clamps_threshold_above_one() {
        let buffer = OutputBuffer::new(100, 1.5);
        assert!((buffer.offer_threshold - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_new_clamps_threshold_below_zero() {
        let buffer = OutputBuffer::new(100, -0.5);
        assert!((buffer.offer_threshold - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_fill_level_empty() {
        let buffer = OutputBuffer::new(100, 0.2);
        assert!((buffer.fill_level() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_fill_level_partial() {
        let mut buffer = OutputBuffer::new(100, 0.2);
        buffer.inventory.add_item("Iron Ingot", 30);
        assert!((buffer.fill_level() - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_fill_level_full() {
        let mut buffer = OutputBuffer::new(100, 0.2);
        buffer.inventory.add_item("Iron Ingot", 100);
        assert!((buffer.fill_level() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_fill_level_zero_capacity() {
        let buffer = OutputBuffer::new(0, 0.2);
        // Zero-capacity buffer should report as full to avoid division by zero
        assert!((buffer.fill_level() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_output_buffer_has_items_to_offer_above_threshold() {
        let mut buffer = OutputBuffer::new(100, 0.2);
        buffer.inventory.add_item("Iron Ingot", 30); // 30% full, threshold 20%
        assert!(buffer.has_items_to_offer());
    }

    #[test]
    fn test_output_buffer_has_items_to_offer_at_threshold() {
        let mut buffer = OutputBuffer::new(100, 0.2);
        buffer.inventory.add_item("Iron Ingot", 20); // 20% full, threshold 20%
        assert!(!buffer.has_items_to_offer());
    }

    #[test]
    fn test_output_buffer_has_items_to_offer_below_threshold() {
        let mut buffer = OutputBuffer::new(100, 0.2);
        buffer.inventory.add_item("Iron Ingot", 10); // 10% full, threshold 20%
        assert!(!buffer.has_items_to_offer());
    }

    #[test]
    fn test_output_buffer_has_items_to_offer_one_threshold() {
        let mut buffer = OutputBuffer::new(100, 1.0);
        buffer.inventory.add_item("Iron Ingot", 100); // 100% full, threshold 100%
                                                      // At threshold (not above), should not offer
        assert!(!buffer.has_items_to_offer());
    }

    #[test]
    fn test_output_buffer_has_items_to_offer_empty_with_zero_threshold() {
        let buffer = OutputBuffer::new(100, 0.0);
        // Empty buffer (0.0) is not > 0.0, so nothing to offer
        assert!(!buffer.has_items_to_offer());
    }
}
