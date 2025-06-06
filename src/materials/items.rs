use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::scene::ron;

pub type ItemId = u32;

#[derive(Component)]
pub struct Item {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub tier: u32,
    // pub stack_size: u32 (not needed yet)
}

#[derive(Resource)]
pub struct ItemRegistry {
    pub definitions: HashMap<ItemId, ItemDef>,
}

impl ItemRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<ItemDef> = ron::from_str(ron_content)?;
        
        let mut definitions = HashMap::new();
        
        for def in definitions_vec {
            definitions.insert(def.id, def);
        }
        
        Ok(Self { definitions })
    }

    pub fn load_from_assets() -> Self {
        let ron_content = include_str!("../assets/items.ron");
        Self::from_ron(ron_content).expect("Failed to load item definitions")
    }

    pub fn get_definition(&self, item_id: ItemId) -> Option<&ItemDef> {
        self.definitions.get(&item_id)
    }

    pub fn create_item(&self, item_id: ItemId) -> Option<Item> {
        self.get_definition(item_id).map(|def| Item { id: def.id })
    }

    // TODO: Add methods for accessing individual item fields from definitions
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InventoryTypes {
    #[default]
    Storage,
    Sender,
    Requester,
    Carrier
} 

#[derive(Component, Default, Serialize, Deserialize, Debug, Clone)]
pub struct InventoryType(pub InventoryTypes);


#[derive(Component)]
#[require(InventoryType)]
pub struct Inventory {
    pub items: HashMap<ItemId, u32>, 
    //TODO: Enforce capacity
    pub capacity: u32,
}

impl Inventory {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }

    pub fn add_item(&mut self, item_id: ItemId, quantity: u32) -> u32 {
        *self.items.entry(item_id).or_insert(0) += quantity;
        quantity
    }

    pub fn remove_item(&mut self, item_id: ItemId, quantity: u32) -> u32 {
        if let Some(current_quantity) = self.items.get_mut(&item_id) {
            let removed = (*current_quantity).min(quantity);
            *current_quantity -= removed;
            if *current_quantity == 0 {
                self.items.remove(&item_id);
            }
            removed
        } else {
            0
        }
    }

    // TODO: Update producers to set operational = false when inventory is full
    pub fn is_full(&self) -> bool {
        let current_quantity = self.items.values().sum::<u32>();
        current_quantity >= self.capacity
    }

    pub fn get_all_items(&self) -> HashMap<ItemId, u32> {
        self.items.clone()
    }

    pub fn get_item_quantity(&self, item_id: u32) -> u32 {
        self.items.iter()
            .find(|(item, _)| **item == item_id)
            .map(|(_, quantity)| *quantity)
            .unwrap_or(0)
    }

    pub fn has_item(&self, item_id: u32, required_quantity: u32) -> bool {
        self.get_item_quantity(item_id) >= required_quantity
    }
}

pub fn transfer_items(
    sender: Entity,
    receiver: Entity,
    inventories: &mut Query<&mut Inventory>,
) {
    if sender == receiver {
        return;
    }
   
    let available_items = if let Ok(sender_inv) = inventories.get(sender) {
        sender_inv.get_all_items()
    } else {
        return;
    };
    
    if available_items.is_empty() {
        return;
    }
    
    for (item_id, quantity) in available_items.into_iter() {
        if quantity == 0 {
            continue;
        }
        
        let removed = if let Ok(mut sender_inv) = inventories.get_mut(sender) {
            sender_inv.remove_item(item_id, quantity)
        } else {
            continue;
        };
        
        if removed > 0 {
            if let Ok(mut receiver_inv) = inventories.get_mut(receiver) {
                receiver_inv.add_item(item_id, removed);
            }
        }
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