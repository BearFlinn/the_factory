use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::{
    grid::{Layer, Position},
    materials::items::{Inventory, InventoryType, InventoryTypes},
    structures::*,
    systems::Operational,
};
use crate::{materials::RecipeDef, systems::Scanner};

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
    pub color: (f32, f32, f32, f32),    // RGBA
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
    PowerConsumer {
        amount: i32,
    },
    PowerGenerator {
        amount: i32,
    },
    ComputeGenerator {
        amount: i32,
    },
    ComputeConsumer {
        amount: i32,
    },
    Inventory {
        capacity: u32,
    },
    InventoryType {
        inv_type: InventoryTypes,
    },
    ViewRange {
        radius: i32,
    },
    NetWorkComponent,
    RecipeCrafter {
        recipe_name: Option<String>,
        available_recipes: Option<Vec<String>>,
        interval: f32,
    },
    Scanner {
        base_scan_interval: f32, // Removed max_radius, simplified to just timing
    },
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
    ) -> Option<Entity> {
        let def = self.get_definition(building_name)?;
        let mut entity_commands = commands.spawn((
            Building,
            def.category,
            Name::new(def.name.clone()),
            Position {
                x: grid_x,
                y: grid_y,
            },
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
            cost: def.placement.cost.to_recipe_def(),
        });

        if let Some((width, height)) = def.appearance.multi_cell {
            entity_commands.insert(MultiCellBuilding {
                width,
                height,
                center_x: grid_x,
                center_y: grid_y,
            });
        }

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
                }
                BuildingComponentDef::NetWorkComponent => {
                    entity_commands.insert(NetWorkComponent);
                }
                BuildingComponentDef::RecipeCrafter {
                    recipe_name,
                    available_recipes,
                    interval,
                } => {
                    let (current_recipe, available_recipes_vec) =
                        match (recipe_name, available_recipes) {
                            // Single-recipe crafter: fixed recipe, empty available list
                            (Some(recipe), None) => (Some(recipe.clone()), Vec::new()),

                            // Multi-recipe crafter: no current recipe, provided available list
                            (None, Some(recipes)) => (None, recipes.clone()),

                            // Invalid configurations - handle gracefully
                            (Some(recipe), Some(recipes)) => {
                                // If both are provided, treat as multi-recipe with the single recipe pre-selected
                                // This provides a migration path for existing configurations
                                (Some(recipe.clone()), recipes.clone())
                            }

                            // Neither provided - create empty crafter (should probably be avoided)
                            (None, None) => (None, Vec::new()),
                        };

                    entity_commands.insert(RecipeCrafter {
                        current_recipe,
                        available_recipes: available_recipes_vec,
                        timer: Timer::from_seconds(*interval, TimerMode::Repeating),
                    });
                }
                BuildingComponentDef::Scanner { base_scan_interval } => {
                    entity_commands.insert(Scanner::new(
                        *base_scan_interval,
                        Position {
                            x: grid_x,
                            y: grid_y,
                        },
                    ));
                }
            }
        }

        let entity = entity_commands.id();
        Some(entity)
    }
}
