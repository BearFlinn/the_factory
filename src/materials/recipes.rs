use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::scene::ron;
use crate::materials::items::ItemName;

pub type RecipeName = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecipeDef {
    pub name: String,
    pub inputs: Vec<(ItemName, u32)>,
    pub outputs: Vec<(ItemName, u32)>,
    pub crafting_time: f32
}

#[derive(Clone)]
#[allow(dead_code)] // TODO: Dynamic recipes
pub enum RecipeType {
    Static(RecipeName),
    Dynamic(RecipeDef),
}

#[derive(Resource)]
pub struct RecipeRegistry {
    definitions: HashMap<RecipeName, RecipeDef>
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

    pub fn load_from_assets() -> Self {
        let ron_content = include_str!("../assets/recipes.ron");
        Self::from_ron(ron_content).expect("Failed to load recipe definitions")
    }

    pub fn get_definition(&self, recipe_name: &str) -> Option<&RecipeDef> {
        self.definitions.get(recipe_name)
    }
}
