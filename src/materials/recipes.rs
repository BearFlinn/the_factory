use crate::materials::items::ItemName;
use bevy::prelude::*;
use bevy::scene::ron;
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
