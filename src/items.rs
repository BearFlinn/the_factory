use bevy::prelude::*;

#[derive(Component)]
pub struct ItemMarker;

#[derive(Component)]
pub struct Item {
    pub id: u32,
    pub name: String,
}


#[derive(Component)]
pub struct Inventory {
    pub items: Vec<(Item, u32)>,
    //TODO: Enforce capacity
    pub capacity: u32,
}

impl Inventory {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: Vec::new(),
            capacity,
        }
    }

    pub fn add_item(&mut self, item: Item, quantity: u32) -> u32 {
        // Find existing item by id
        if let Some((_, existing_quantity)) = self.items.iter_mut()
            .find(|(existing_item, _)| existing_item.id == item.id) {
            *existing_quantity += quantity;
            return quantity;
        }
        
        // Add new item if not found
        self.items.push((item, quantity));
        quantity
    }

    pub fn remove_item(&mut self, item_id: u32, quantity: u32) -> u32 {
        if let Some(index) = self.items.iter()
            .position(|(item, _)| item.id == item_id) {
            let (_, current_quantity) = &mut self.items[index];
            let removed = (*current_quantity).min(quantity);
            *current_quantity -= removed;
            
            // Remove item if quantity reaches 0
            if *current_quantity == 0 {
                self.items.remove(index);
            }
            
            return removed;
        }
        0
    }

    pub fn get_item_quantity(&self, item_id: u32) -> u32 {
        self.items.iter()
            .find(|(item, _)| item.id == item_id)
            .map(|(_, quantity)| *quantity)
            .unwrap_or(0)
    }

    pub fn has_item(&self, item_id: u32, required_quantity: u32) -> bool {
        self.get_item_quantity(item_id) >= required_quantity
    }
}

// Helper function to create standard items
pub fn create_ore_item() -> Item {
    Item {
        id: 0,
        name: "Ore".to_string(),
    }
}

pub fn transfer_items(
    sender: Entity,
    receiver: Entity,
    inventories: &mut Query<&mut Inventory>,
) {
    // We need to work around the borrow checker by getting both mutable references safely
    if sender == receiver {
        return; // Can't transfer to self
    }
    
    // Get the available ore from sender first
    let available_ore = if let Ok(sender_inv) = inventories.get(sender) {
        sender_inv.get_item_quantity(0)
    } else {
        return;
    };
    
    if available_ore == 0 {
        return;
    }
    
    // Remove from sender
    let removed = if let Ok(mut sender_inv) = inventories.get_mut(sender) {
        sender_inv.remove_item(0, available_ore)
    } else {
        return;
    };
    
    // Add to receiver
    if removed > 0 {
        if let Ok(mut receiver_inv) = inventories.get_mut(receiver) {
            receiver_inv.add_item(
                create_ore_item(), 
                removed
            );
            println!("Transferred {} ore", removed);
        }
    }
}

pub struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
    }
}