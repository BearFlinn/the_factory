use bevy::prelude::*;
use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::grid::{Position, Layer};
use crate::items::InventoryTypes;
pub use crate::systems::Operational;
pub use crate::structures::*;
pub use crate::items::{Inventory, InventoryType};

// Numerical building IDs for performance and type safety
pub type BuildingId = u32;

// Constants for building IDs - can be moved to separate file if needed
pub const HUB: BuildingId = 0;
pub const MINING_DRILL: BuildingId = 1;
pub const CONNECTOR: BuildingId = 2;
pub const RADAR: BuildingId = 3;
pub const GENERATOR: BuildingId = 4;
pub const DATACENTER: BuildingId = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Component, Serialize, Deserialize)]
pub enum BuildingCategory {
    Production,  // Harvesters, Generators, Datacenters
    Logistics,   // Connectors, Transport
    Utility,     // Radar, Defense
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildingDef {
    pub id: BuildingId,
    pub category: BuildingCategory,
    pub appearance: AppearanceDef,
    pub placement: PlacementDef,
    pub components: Vec<ComponentDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceDef {
    pub name: String,
    pub size: (f32, f32),
    pub color: (f32, f32, f32, f32), // RGBA
    pub multi_cell: Option<(i32, i32)>, // (width, height)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlacementDef {
    pub cost: Option<CostDef>,
    pub rules: Vec<PlacementRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CostDef {
    pub ore: u32,
    // Future: could add more resource types
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComponentDef {
    Producer { amount: u32, interval: f32 },
    PowerConsumer { amount: i32 },
    PowerGenerator { amount: i32 },
    ComputeGenerator { amount: i32 },
    ComputeConsumer { amount: i32 },
    ResourceConsumer { amount: u32, interval: f32 },
    Inventory { capacity: u32 },
    InventoryType { inv_type: InventoryTypes },
    ViewRange { radius: i32 },
    NetWorkComponent,
}

/// Registry that loads building definitions from RON files
#[derive(Resource)]
pub struct BuildingRegistry {
    pub definitions: HashMap<BuildingId, BuildingDef>,
    // Optional: maintain name lookup for UI/debugging
    pub name_to_id: HashMap<String, BuildingId>,
}

impl BuildingRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<BuildingDef> = ron::from_str(ron_content)?;
        
        let mut definitions = HashMap::new();
        let mut name_to_id = HashMap::new();
        
        for def in definitions_vec {
            name_to_id.insert(def.appearance.name.clone(), def.id);
            definitions.insert(def.id, def);
        }
        
        Ok(Self { definitions, name_to_id })
    }

    pub fn get_definition(&self, building_id: BuildingId) -> Option<&BuildingDef> {
        self.definitions.get(&building_id)
    }

    pub fn get_definition_by_name(&self, name: &str) -> Option<&BuildingDef> {
        let id = self.name_to_id.get(name)?;
        self.definitions.get(id)
    }

    pub fn get_all_building_ids(&self) -> Vec<BuildingId> {
        self.definitions.keys().cloned().collect()
    }

    pub fn get_buildings_by_category(&self, category: BuildingCategory) -> Vec<BuildingId> {
        self.definitions
            .iter()
            .filter(|(_, def)| def.category == category)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn get_name_by_id(&self, building_id: BuildingId) -> Option<String> {
        let def = self.get_definition(building_id)?; 
        Some(def.appearance.name.clone())
    }

    /// Spawn a building entity with all its components
    pub fn spawn_building(
        &self,
        commands: &mut Commands,
        building_id: BuildingId,
        grid_x: i32,
        grid_y: i32,
        world_pos: Vec2,
    ) -> Option<(Entity, i32)> {
        let def = self.get_definition(building_id)?;
        println!("Spawning building: {} With Components {}", def.appearance.name, def.components.iter().map(|c| format!("{:?}", c)).collect::<Vec<String>>().join(", "));
        // Start with base building components
        let mut entity_commands = commands.spawn((
            Building { id: building_id },
            def.category,
            construction::Name { name: def.appearance.name.clone().to_string() },
            Position { x: grid_x, y: grid_y },
            Operational(false),
            Layer(BUILDING_LAYER),
            Sprite::from_color(
                Color::srgba(
                    def.appearance.color.0,
                    def.appearance.color.1,
                    def.appearance.color.2,
                    def.appearance.color.3,
                ),
                Vec2::new(def.appearance.size.0, def.appearance.size.1),
            ),
            Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
        ));

        // Add cost component if specified
        if let Some(cost) = &def.placement.cost {
            entity_commands.insert(BuildingCost { ore: cost.ore });
        }

        // Add multi-cell component if specified
        if let Some((width, height)) = def.appearance.multi_cell {
            entity_commands.insert(MultiCellBuilding {
                width,
                height,
                center_x: grid_x,
                center_y: grid_y,
            });
        }

        // Track view radius for return value
        let mut view_radius = 0;

        // Add dynamic components based on definition
        for component in &def.components {
            match component {
                ComponentDef::Producer { amount, interval } => {
                    entity_commands.insert(Producer {
                        amount: *amount,
                        timer: Timer::from_seconds(*interval, TimerMode::Repeating),
                    });
                }
                ComponentDef::PowerConsumer { amount } => {
                    entity_commands.insert(PowerConsumer { amount: *amount });
                }
                ComponentDef::PowerGenerator { amount } => {
                    entity_commands.insert(PowerGenerator { amount: *amount });
                }
                ComponentDef::ComputeGenerator { amount } => {
                    entity_commands.insert(ComputeGenerator { amount: *amount });
                }
                ComponentDef::ComputeConsumer { amount } => {
                    entity_commands.insert(ComputeConsumer { amount: *amount });
                }
                ComponentDef::ResourceConsumer { amount, interval } => {
                    entity_commands.insert(ResourceConsumer {
                        amount: *amount,
                        timer: Timer::from_seconds(*interval, TimerMode::Repeating),
                    });
                }
                ComponentDef::Inventory { capacity } => {
                    entity_commands.insert(Inventory::new(*capacity));
                }
                ComponentDef::InventoryType { inv_type } => {
                    entity_commands.insert(InventoryType(inv_type.clone()));
                }
                ComponentDef::ViewRange { radius } => {
                    entity_commands.insert(ViewRange { radius: *radius });
                    view_radius = *radius;
                }
                ComponentDef::NetWorkComponent => {
                    entity_commands.insert(NetWorkComponent);
                }
            }
        }

        let entity = entity_commands.id();
        Some((entity, view_radius))
    }
}
