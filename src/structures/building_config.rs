use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{materials::RecipeDef, systems::OperationalCondition};
pub use crate::{
    grid::{Position, Layer},
    systems::Operational,
    structures::*,
    materials::items::{Inventory, InventoryType, InventoryTypes},
};

pub type BuildingName = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Component, Serialize, Deserialize)]
pub enum BuildingCategory {
    Production,
    Logistics,
    Utility,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildingDef {
    pub name: String,
    pub category: BuildingCategory,
    pub appearance: AppearanceDef,
    pub placement: PlacementDef,
    pub components: Vec<BuildingComponentDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceDef {
    pub size: (f32, f32),
    pub color: (f32, f32, f32, f32), // RGBA
    pub multi_cell: Option<(i32, i32)>, // (width, height)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlacementDef {
    pub cost: CostDef,
    pub rules: Vec<PlacementRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CostDef {
    pub inputs: HashMap<String, u32>,
    pub crafting_time: f32,
}

impl CostDef {
    pub fn to_recipe_def(&self) -> RecipeDef {
        RecipeDef {
            name: String::new(),
            inputs: self.inputs.clone(),
            outputs: HashMap::new(),
            crafting_time: self.crafting_time,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BuildingComponentDef {
    PowerConsumer { amount: i32 },
    PowerGenerator { amount: i32 },
    ComputeGenerator { amount: i32 },
    ComputeConsumer { amount: i32 },
    Inventory { capacity: u32 },
    InventoryType { inv_type: InventoryTypes },
    ViewRange { radius: i32 },
    NetWorkComponent,
    RecipeCrafter { recipe_name: String, interval: f32 },
}

#[derive(Resource)]
pub struct BuildingRegistry {
    pub definitions: HashMap<BuildingName, BuildingDef>,
}

impl BuildingRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<BuildingDef> = ron::from_str(ron_content)?;
        
        let mut definitions = HashMap::new();
        
        for def in definitions_vec {
            definitions.insert(def.name.clone(), def);
        }
        
        Ok(Self { definitions })
    }

    pub fn get_definition(&self, building_name: &str) -> Option<&BuildingDef> {
        self.definitions.get(building_name)
    }

    pub fn get_all_building_names(&self) -> Vec<BuildingName> {
        self.definitions.keys().cloned().collect()
    }

    pub fn get_buildings_by_category(&self, category: BuildingCategory) -> Vec<BuildingName> {
        self.definitions
            .iter()
            .filter(|(_, def)| def.category == category)
            .map(|(name, _)| name.clone())
            .collect()
    }

    #[allow(dead_code)] // Is used in spawn_building_buttons, rust analyzer broky
    pub fn get_name_by_name(&self, building_name: &str) -> Option<String> {
        let def = self.get_definition(building_name)?; 
        Some(def.name.clone())
    }

    pub fn spawn_building(
        &self,
        commands: &mut Commands,
        building_name: &str,
        grid_x: i32,
        grid_y: i32,
        world_pos: Vec2,
    ) -> Option<(Entity, i32)> {
        let def = self.get_definition(building_name)?;
        let mut entity_commands = commands.spawn((
            Building,
            def.category,
            Name::new(format!("{}",&def.name)),
            Position { x: grid_x, y: grid_y },
            Operational(Some(Vec::new())),
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

        entity_commands.insert(BuildingCost { 
            cost: def.placement.cost.to_recipe_def() 
        });

        if let Some((width, height)) = def.appearance.multi_cell {
            entity_commands.insert(MultiCellBuilding {
                width,
                height,
                center_x: grid_x,
                center_y: grid_y,
            });
        }

        let mut view_radius = 0;

        for component in &def.components {
            match component {
                BuildingComponentDef::PowerConsumer { amount } => {
                    entity_commands.insert(PowerConsumer { amount: *amount });
                }
                BuildingComponentDef::PowerGenerator { amount } => {
                    entity_commands.insert(PowerGenerator { amount: *amount });
                }
                BuildingComponentDef::ComputeGenerator { amount } => {
                    entity_commands.insert(ComputeGenerator { amount: *amount });
                }
                BuildingComponentDef::ComputeConsumer { amount } => {
                    entity_commands.insert(ComputeConsumer { amount: *amount });
                }
                BuildingComponentDef::Inventory { capacity } => {
                    entity_commands.insert(Inventory::new(*capacity));
                }
                BuildingComponentDef::InventoryType { inv_type } => {
                    entity_commands.insert(InventoryType(inv_type.clone()));
                }
                BuildingComponentDef::ViewRange { radius } => {
                    entity_commands.insert(ViewRange { radius: *radius });
                    view_radius = *radius;
                }
                BuildingComponentDef::NetWorkComponent => {
                    entity_commands.insert(NetWorkComponent);
                }
                BuildingComponentDef::RecipeCrafter { recipe_name, interval } => {
                    entity_commands.insert(RecipeCrafter { 
                        recipe: recipe_name.clone(),
                        timer: Timer::from_seconds(*interval, TimerMode::Repeating),
                     });
                }
            }
        }

        let entity = entity_commands.id();
        Some((entity, view_radius))
    }
}
