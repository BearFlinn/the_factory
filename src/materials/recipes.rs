use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bevy::scene::ron;
use crate::materials::items::ItemId;

pub type RecipeId = u32;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecipeDef {
    pub inputs: Vec<(ItemId, u32)>,
    pub outputs: Vec<(ItemId, u32)>,
    pub crafting_time: f32
}

#[derive(Resource)]
pub struct RecipeRegistry {
    definitions: HashMap<RecipeId, RecipeDef>
}

impl RecipeRegistry {
    pub fn from_ron(ron_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let definitions_vec: Vec<RecipeDef> = ron::from_str(ron_content)?;
        
        let mut definitions = HashMap::new();
        
        for def in definitions_vec {
            definitions.insert(def.inputs[0].0, def);
        }
        
        Ok(Self { definitions })
    }

    pub fn load_from_assets() -> Self {
        let ron_content = include_str!("../assets/recipes.ron");
        Self::from_ron(ron_content).expect("Failed to load recipe definitions")
    }
}