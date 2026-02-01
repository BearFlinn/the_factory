use crate::materials::items::ItemName;
use bevy::prelude::*;
use ron;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type RecipeName = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecipeDef {
    pub name: String,
    pub inputs: HashMap<ItemName, u32>,
    pub outputs: HashMap<ItemName, u32>,
    pub crafting_time: f32,
}

#[derive(Clone)]
#[allow(dead_code)] // TODO: Dynamic recipes
pub enum RecipeType {
    Static(RecipeName),
    Dynamic(RecipeDef),
}

#[derive(Resource)]
pub struct RecipeRegistry {
    definitions: HashMap<RecipeName, RecipeDef>,
}

impl RecipeRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<RecipeDef> = ron::from_str(ron_content)?;

        let mut definitions = HashMap::new();

        for def in definitions_vec {
            definitions.insert(def.name.clone(), def);
        }

        Ok(Self { definitions })
    }

    /// Load recipe definitions from embedded assets.
    ///
    /// # Errors
    /// Returns an error if the embedded RON content fails to parse.
    pub fn load_from_assets() -> Result<Self, Box<dyn std::error::Error>> {
        let ron_content = include_str!("../assets/recipes.ron");
        Self::from_ron(ron_content)
    }

    pub fn get_definition(&self, recipe_name: &str) -> Option<&RecipeDef> {
        self.definitions.get(recipe_name)
    }

    pub fn get_inputs(&self, recipe_name: &str) -> Option<&HashMap<ItemName, u32>> {
        self.definitions.get(recipe_name).map(|def| &def.inputs)
    }

    pub fn get_outputs(&self, recipe_name: &str) -> Option<&HashMap<ItemName, u32>> {
        self.definitions.get(recipe_name).map(|def| &def.outputs)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::float_cmp)]

    use super::*;

    #[test]
    fn test_recipe_registry_from_ron_valid() {
        let ron_content = r#"[
            (
                name: "Test Recipe",
                inputs: {"Iron Ore": 2, "Coal": 1},
                outputs: {"Iron Ingot": 1},
                crafting_time: 2.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        assert_eq!(registry.definitions.len(), 1);
        assert!(registry.definitions.contains_key("Test Recipe"));
    }

    #[test]
    fn test_recipe_registry_from_ron_empty() {
        let ron_content = "[]";
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        assert!(registry.definitions.is_empty());
    }

    #[test]
    fn test_recipe_registry_from_ron_multiple_recipes() {
        let ron_content = r#"[
            (
                name: "Recipe A",
                inputs: {"Item1": 1},
                outputs: {"Item2": 1},
                crafting_time: 1.0,
            ),
            (
                name: "Recipe B",
                inputs: {"Item2": 2},
                outputs: {"Item3": 1},
                crafting_time: 2.5,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        assert_eq!(registry.definitions.len(), 2);
        assert!(registry.definitions.contains_key("Recipe A"));
        assert!(registry.definitions.contains_key("Recipe B"));
    }

    #[test]
    fn test_recipe_registry_from_ron_invalid() {
        let ron_content = "not valid ron";
        let result = RecipeRegistry::from_ron(ron_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_recipe_registry_from_ron_missing_field() {
        let ron_content = r#"[
            (
                name: "Incomplete Recipe",
                inputs: {"Item": 1},
            ),
        ]"#;
        let result = RecipeRegistry::from_ron(ron_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_definition_existing() {
        let ron_content = r#"[
            (
                name: "Test Recipe",
                inputs: {"Iron Ore": 2},
                outputs: {"Iron Ingot": 1},
                crafting_time: 3.5,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Test Recipe");
        assert!(def.is_some());
        let def = def.unwrap();
        assert_eq!(def.name, "Test Recipe");
        assert_eq!(def.crafting_time, 3.5);
        assert_eq!(def.inputs.get("Iron Ore"), Some(&2));
        assert_eq!(def.outputs.get("Iron Ingot"), Some(&1));
    }

    #[test]
    fn test_get_definition_nonexistent() {
        let ron_content = "[]";
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Nonexistent");
        assert!(def.is_none());
    }

    #[test]
    fn test_get_inputs_existing() {
        let ron_content = r#"[
            (
                name: "Multi Input Recipe",
                inputs: {"Item A": 3, "Item B": 5},
                outputs: {"Result": 1},
                crafting_time: 1.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let inputs = registry.get_inputs("Multi Input Recipe");
        assert!(inputs.is_some());
        let inputs = inputs.unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs.get("Item A"), Some(&3));
        assert_eq!(inputs.get("Item B"), Some(&5));
    }

    #[test]
    fn test_get_inputs_empty() {
        let ron_content = r#"[
            (
                name: "No Input Recipe",
                inputs: {},
                outputs: {"Output": 1},
                crafting_time: 1.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let inputs = registry.get_inputs("No Input Recipe");
        assert!(inputs.is_some());
        let inputs = inputs.unwrap();
        assert!(inputs.is_empty());
    }

    #[test]
    fn test_get_inputs_nonexistent_recipe() {
        let ron_content = "[]";
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let inputs = registry.get_inputs("Nonexistent");
        assert!(inputs.is_none());
    }

    #[test]
    fn test_get_outputs_existing() {
        let ron_content = r#"[
            (
                name: "Multi Output Recipe",
                inputs: {"Input": 1},
                outputs: {"Output A": 2, "Output B": 3},
                crafting_time: 1.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let outputs = registry.get_outputs("Multi Output Recipe");
        assert!(outputs.is_some());
        let outputs = outputs.unwrap();
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs.get("Output A"), Some(&2));
        assert_eq!(outputs.get("Output B"), Some(&3));
    }

    #[test]
    fn test_get_outputs_empty() {
        let ron_content = r#"[
            (
                name: "No Output Recipe",
                inputs: {"Input": 1},
                outputs: {},
                crafting_time: 1.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let outputs = registry.get_outputs("No Output Recipe");
        assert!(outputs.is_some());
        let outputs = outputs.unwrap();
        assert!(outputs.is_empty());
    }

    #[test]
    fn test_get_outputs_nonexistent_recipe() {
        let ron_content = "[]";
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let outputs = registry.get_outputs("Nonexistent");
        assert!(outputs.is_none());
    }

    #[test]
    fn test_recipe_structure_integrity() {
        let ron_content = r#"[
            (
                name: "Full Recipe",
                inputs: {"Raw Material": 10},
                outputs: {"Product": 5},
                crafting_time: 4.5,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Full Recipe").unwrap();

        assert_eq!(def.name, "Full Recipe");
        assert_eq!(def.inputs.len(), 1);
        assert_eq!(def.outputs.len(), 1);
        assert!((def.crafting_time - 4.5).abs() < f32::EPSILON);
        assert!(def.inputs.contains_key("Raw Material"));
        assert!(def.outputs.contains_key("Product"));
    }

    #[test]
    fn test_recipe_with_zero_crafting_time() {
        let ron_content = r#"[
            (
                name: "Instant Recipe",
                inputs: {"Input": 1},
                outputs: {"Output": 1},
                crafting_time: 0.0,
            ),
        ]"#;
        let registry = RecipeRegistry::from_ron(ron_content).unwrap();
        let def = registry.get_definition("Instant Recipe").unwrap();
        assert!((def.crafting_time).abs() < f32::EPSILON);
    }
}
