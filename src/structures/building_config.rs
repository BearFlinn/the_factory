use bevy::scene::ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::{
    grid::{Layer, Position},
    materials::items::{InputPort, Inventory, OutputPort, StoragePort},
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
    // Legacy inventory component (used by Hub)
    Inventory {
        capacity: u32,
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
    // Port-based components
    InputPort {
        capacity: u32,
    },
    OutputPort {
        capacity: u32,
    },
    StoragePort {
        capacity: u32,
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

    // The line count is high due to the match statement mapping config variants to components.
    // Each arm is simple and the structure is readable despite the length.
    #[allow(clippy::too_many_lines)]
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
                // Port-based components
                BuildingComponentDef::InputPort { capacity } => {
                    entity_commands.insert(InputPort::new(*capacity));
                }
                BuildingComponentDef::OutputPort { capacity } => {
                    entity_commands.insert(OutputPort::new(*capacity));
                }
                BuildingComponentDef::StoragePort { capacity } => {
                    entity_commands.insert(StoragePort::new(*capacity));
                }
            }
        }

        let entity = entity_commands.id();
        Some(entity)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const VALID_BUILDING_RON: &str = r#"[
        (
            name: "TestHub",
            category: Utility,
            appearance: (
                size: (48.0, 48.0),
                color: (0.3, 0.3, 0.8, 1.0),
                multi_cell: None,
            ),
            placement: (
                cost: (
                    inputs: {},
                    crafting_time: 0.0,
                ),
                rules: [],
            ),
            components: [
                PowerGenerator(amount: 100),
                ComputeGenerator(amount: 50),
                Inventory(capacity: 500),
                ViewRange(radius: 5),
                NetWorkComponent,
            ],
        ),
        (
            name: "TestMiner",
            category: Production,
            appearance: (
                size: (40.0, 40.0),
                color: (0.6, 0.4, 0.2, 1.0),
                multi_cell: None,
            ),
            placement: (
                cost: (
                    inputs: {"iron_plate": 5},
                    crafting_time: 2.0,
                ),
                rules: [RequiresResource],
            ),
            components: [
                PowerConsumer(amount: 10),
                Inventory(capacity: 20),
                RecipeCrafter(
                    recipe_name: Some("mine_iron"),
                    available_recipes: None,
                    interval: 1.0,
                ),
            ],
        ),
        (
            name: "TestConnector",
            category: Logistics,
            appearance: (
                size: (32.0, 32.0),
                color: (0.5, 0.5, 0.5, 1.0),
                multi_cell: None,
            ),
            placement: (
                cost: (
                    inputs: {"iron_plate": 2},
                    crafting_time: 1.0,
                ),
                rules: [AdjacentToNetwork],
            ),
            components: [
                NetWorkComponent,
            ],
        ),
    ]"#;

    const MULTI_CELL_BUILDING_RON: &str = r#"[
        (
            name: "LargeFactory",
            category: Production,
            appearance: (
                size: (96.0, 64.0),
                color: (0.4, 0.4, 0.6, 1.0),
                multi_cell: Some((2, 3)),
            ),
            placement: (
                cost: (
                    inputs: {"iron_plate": 20, "copper_plate": 10},
                    crafting_time: 5.0,
                ),
                rules: [AdjacentToNetwork],
            ),
            components: [
                PowerConsumer(amount: 50),
                ComputeConsumer(amount: 25),
                Inventory(capacity: 100),
            ],
        ),
    ]"#;

    const SCANNER_BUILDING_RON: &str = r#"[
        (
            name: "Radar",
            category: Utility,
            appearance: (
                size: (44.0, 44.0),
                color: (0.2, 0.7, 0.3, 1.0),
                multi_cell: None,
            ),
            placement: (
                cost: (
                    inputs: {"iron_plate": 8},
                    crafting_time: 3.0,
                ),
                rules: [],
            ),
            components: [
                PowerConsumer(amount: 30),
                Scanner(base_scan_interval: 2.5),
            ],
        ),
    ]"#;

    #[test]
    fn from_ron_with_valid_building_data() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        assert_eq!(registry.definitions.len(), 3);
        assert!(registry.definitions.contains_key("TestHub"));
        assert!(registry.definitions.contains_key("TestMiner"));
        assert!(registry.definitions.contains_key("TestConnector"));
    }

    #[test]
    fn from_ron_with_invalid_data_returns_error() {
        let invalid_ron = r#"[
            (
                name: "Broken",
                invalid_field: "whoops",
            ),
        ]"#;

        let result = BuildingRegistry::from_ron(invalid_ron);
        assert!(result.is_err());
    }

    #[test]
    fn from_ron_with_malformed_syntax_returns_error() {
        let malformed_ron = r"[ not valid ron syntax {";

        let result = BuildingRegistry::from_ron(malformed_ron);
        assert!(result.is_err());
    }

    #[test]
    fn get_definition_for_existing_building() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let hub_def = registry.get_definition("TestHub");
        assert!(hub_def.is_some());

        let hub = hub_def.unwrap();
        assert_eq!(hub.name, "TestHub");
        assert_eq!(hub.category, BuildingCategory::Utility);
        assert_eq!(hub.appearance.size, (48.0, 48.0));
        assert_eq!(hub.appearance.color, (0.3, 0.3, 0.8, 1.0));
    }

    #[test]
    fn get_definition_for_nonexistent_building_returns_none() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let result = registry.get_definition("NonExistentBuilding");
        assert!(result.is_none());
    }

    #[test]
    fn get_all_building_names_returns_all_registered() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let names = registry.get_all_building_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"TestHub".to_string()));
        assert!(names.contains(&"TestMiner".to_string()));
        assert!(names.contains(&"TestConnector".to_string()));
    }

    #[test]
    fn get_buildings_by_category_filters_correctly() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let production_buildings = registry.get_buildings_by_category(BuildingCategory::Production);
        assert_eq!(production_buildings.len(), 1);
        assert!(production_buildings.contains(&"TestMiner".to_string()));

        let utility_buildings = registry.get_buildings_by_category(BuildingCategory::Utility);
        assert_eq!(utility_buildings.len(), 1);
        assert!(utility_buildings.contains(&"TestHub".to_string()));

        let logistics_buildings = registry.get_buildings_by_category(BuildingCategory::Logistics);
        assert_eq!(logistics_buildings.len(), 1);
        assert!(logistics_buildings.contains(&"TestConnector".to_string()));
    }

    #[test]
    fn cost_def_to_recipe_def_conversion() {
        let mut inputs = HashMap::new();
        inputs.insert("iron_plate".to_string(), 10);
        inputs.insert("copper_plate".to_string(), 5);

        let cost = CostDef {
            inputs,
            crafting_time: 3.5,
        };

        let recipe = cost.to_recipe_def();

        assert_eq!(recipe.name, "");
        assert_eq!(recipe.inputs.get("iron_plate"), Some(&10));
        assert_eq!(recipe.inputs.get("copper_plate"), Some(&5));
        assert!(recipe.outputs.is_empty());
        assert!((recipe.crafting_time - 3.5).abs() < f32::EPSILON);
    }

    #[test]
    fn building_component_definitions_parsing() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let hub = registry.get_definition("TestHub").unwrap();
        assert_eq!(hub.components.len(), 5);
        assert!(hub
            .components
            .contains(&BuildingComponentDef::PowerGenerator { amount: 100 }));
        assert!(hub
            .components
            .contains(&BuildingComponentDef::ComputeGenerator { amount: 50 }));
        assert!(hub
            .components
            .contains(&BuildingComponentDef::Inventory { capacity: 500 }));
        assert!(hub
            .components
            .contains(&BuildingComponentDef::ViewRange { radius: 5 }));
        assert!(hub
            .components
            .contains(&BuildingComponentDef::NetWorkComponent));

        let miner = registry.get_definition("TestMiner").unwrap();
        assert!(miner
            .components
            .contains(&BuildingComponentDef::PowerConsumer { amount: 10 }));
    }

    #[test]
    fn multi_cell_building_parsing() {
        let registry = BuildingRegistry::from_ron(MULTI_CELL_BUILDING_RON).unwrap();

        let factory = registry.get_definition("LargeFactory").unwrap();
        assert_eq!(factory.appearance.multi_cell, Some((2, 3)));
        assert_eq!(factory.appearance.size, (96.0, 64.0));
    }

    #[test]
    fn scanner_component_parsing() {
        let registry = BuildingRegistry::from_ron(SCANNER_BUILDING_RON).unwrap();

        let radar = registry.get_definition("Radar").unwrap();

        let has_scanner = radar.components.iter().any(|c| {
            matches!(
                c,
                BuildingComponentDef::Scanner {
                    base_scan_interval: interval
                } if (*interval - 2.5).abs() < f32::EPSILON
            )
        });
        assert!(
            has_scanner,
            "Radar should have Scanner component with base_scan_interval of 2.5"
        );
    }

    #[test]
    fn placement_rules_parsing() {
        let registry = BuildingRegistry::from_ron(VALID_BUILDING_RON).unwrap();

        let miner = registry.get_definition("TestMiner").unwrap();
        assert_eq!(miner.placement.rules.len(), 1);
        assert!(matches!(
            miner.placement.rules[0],
            PlacementRule::RequiresResource
        ));

        let connector = registry.get_definition("TestConnector").unwrap();
        assert_eq!(connector.placement.rules.len(), 1);
        assert!(matches!(
            connector.placement.rules[0],
            PlacementRule::AdjacentToNetwork
        ));

        let hub = registry.get_definition("TestHub").unwrap();
        assert!(hub.placement.rules.is_empty());
    }
}
