use bevy::prelude::*;
use std::collections::HashMap;
use crate::{grid::{CellChildren, Grid, Layer, Position}, resources::Item, structures::{NetworkChangedEvent, BUILDING_LAYER}};

#[derive(Component)]
pub struct Building;

#[derive(Component)]
pub struct Name {
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Component)]
pub enum BuildingType {
    Harvester,
    Connector,
    Generator,
    Datacenter,
    Radar,
}

#[derive(Component)]
pub struct ViewRange {
    pub radius: i32,
}

#[derive(Component)]
pub struct Producer {
    pub amount: u32,
    pub timer: Timer,
}

#[derive(Component)]
pub struct PowerGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct PowerConsumer {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeConsumer {
    pub amount: i32,
}

#[derive(Component)]
pub struct Inventory {
    pub items: Vec<(Item, u32)>,
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

#[derive(Component)]
pub struct ResourceConsumer {
    pub amount: u32,
    pub timer: Timer,
}

// TODO: Update to use receipes
#[derive(Component)]
pub struct BuildingCost {
    pub ore: u32,
}

#[derive(Component)]
pub struct MultiCellBuilding {
    pub width: i32,
    pub height: i32,
    pub center_x: i32,
    pub center_y: i32,
}

#[derive(Component)]
pub struct Operational(pub bool);

#[derive(Clone, Debug)]
pub struct BuildingDefinition {
    pub name: String,
    pub building_type: BuildingType,
    pub size: Vec2,
    pub color: Color,
    pub view_radius: i32,
    pub construction_cost: Option<i32>,
    pub production_rate: Option<u32>,
    pub production_interval: Option<f32>,
    pub power_consumption: Option<i32>,
    pub power_generation: Option<i32>,
    pub compute_generation: Option<i32>,
    pub compute_consumption: Option<i32>,
    pub resource_consumption: Option<(u32, f32)>,
    pub inventory: Option<u32>,
    pub multi_cell: Option<(i32, i32)>,
}

#[derive(Resource)]
pub struct BuildingRegistry {
    definitions: HashMap<String, BuildingDefinition>,
}

impl BuildingRegistry {
    pub fn new() -> Self {
        let mut definitions = HashMap::new();

        definitions.insert("Mining Drill".to_string(), BuildingDefinition {
            name: "Mining Drill".to_string(),
            building_type: BuildingType::Harvester,
            size: Vec2::new(32.0, 32.0),
            color: Color::srgb(0.3, 0.7, 0.3),
            view_radius: 2,
            construction_cost: Some(60),
            production_rate: Some(1),
            production_interval: Some(1.0),
            power_consumption: Some(10),
            power_generation: None,
            compute_generation: None,
            compute_consumption: None,
            resource_consumption: None,
            inventory: Some(100),
            multi_cell: None,
        });
        
        definitions.insert("Connector".to_string(), BuildingDefinition {
            name: "Connector".to_string(),
            building_type: BuildingType::Connector,
            size: Vec2::new(16.0, 16.0),
            color: Color::srgb(0.7, 0.3, 0.7),
            view_radius: 1,
            construction_cost: Some(10),
            production_rate: None,
            production_interval: None,
            power_consumption: None,
            power_generation: None,
            compute_generation: None,
            compute_consumption: None,
            resource_consumption: None,
            inventory: None,
            multi_cell: None,
        });
        
        definitions.insert("Radar".to_string(), BuildingDefinition {
            name: "Radar".to_string(),
            building_type: BuildingType::Radar,
            size: Vec2::new(32.0, 32.0),
            color: Color::srgb(0.7, 0.7, 0.3),
            view_radius: 10,
            construction_cost: Some(150),
            production_rate: None,
            production_interval: None,
            power_consumption: Some(30),
            power_generation: None,
            compute_generation: None,
            compute_consumption: Some(50),
            resource_consumption: None,
            inventory: None,
            multi_cell: None,
        });
        definitions.insert("Generator".to_string(), BuildingDefinition {
            name: "Generator".to_string(),
            building_type: BuildingType::Generator,
            size: Vec2::new(32.0, 32.0),
            color: Color::srgb(0.3, 0.3, 0.7),
            view_radius: 2,
            construction_cost: Some(60),
            production_rate: None,
            production_interval: None,
            power_consumption: None,
            power_generation: Some(40),
            compute_generation: None,
            compute_consumption: None,
            resource_consumption: Some((1, 2.0)), // Consume 1 ore every 2 seconds
            inventory: None,
            multi_cell: None,
        });
        definitions.insert("Datacenter".to_string(), BuildingDefinition {
            name: "Datacenter".to_string(),
            building_type: BuildingType::Datacenter,
            size: Vec2::new(48.0, 48.0),
            color: Color::srgb(0.7, 0.3, 0.8),
            view_radius: 2,
            construction_cost: Some(200),
            production_rate: None,
            production_interval: None,
            power_consumption: Some(100),
            power_generation: None,
            compute_generation: Some(100),
            compute_consumption: None,
            resource_consumption: None,
            inventory: None,
            multi_cell: None,
        });
        
        Self { definitions }
    }
    
    pub fn get_definition(&self, building_name: &str) -> Option<&BuildingDefinition> {
        self.definitions.get(building_name)
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
            def.building_type,
            Name { name: def.name.clone() },
            Position { x: grid_x, y: grid_y },
            ViewRange { radius: def.view_radius },
            Operational(false),
            Layer(BUILDING_LAYER),
            Sprite::from_color(def.color, def.size),
            Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
        ));

        if let Some(construction_cost) = def.construction_cost {
            entity_commands.insert(BuildingCost { ore: construction_cost as u32 });
        }
        
        if let (Some(rate), Some(interval)) = (def.production_rate, def.production_interval) {
            entity_commands.insert(Producer {
                amount: rate,
                timer: Timer::from_seconds(interval, TimerMode::Repeating),
            });
        }

        if let Some(power_consumption) = def.power_consumption {
            entity_commands.insert(PowerConsumer { amount: power_consumption });
        }
        
        if let Some(power_generation) = def.power_generation {
            entity_commands.insert(PowerGenerator { amount: power_generation });

        }
        
        if let Some(compute_generation) = def.compute_generation {
            entity_commands.insert(ComputeGenerator { amount: compute_generation });
        }
        
        if let Some(compute_consumption) = def.compute_consumption {
            entity_commands.insert(ComputeConsumer { amount: compute_consumption });
        }

        if let Some((amount, interval)) = def.resource_consumption {
            entity_commands.insert(ResourceConsumer {
                amount,
                timer: Timer::from_seconds(interval, TimerMode::Repeating),
            });
        }

        if let Some(inventory) = def.inventory {
            entity_commands.insert(Inventory { items: Vec::new(), capacity: inventory });
        }
        
        if let Some((width, height)) = def.multi_cell {
            entity_commands.insert(MultiCellBuilding {
                width,
                height,
                center_x: grid_x,
                center_y: grid_y,
            });
        }
        
        let entity = entity_commands.id();
        Some((entity, def.view_radius))
    }
    
    pub fn get_all_building_names(&self) -> Vec<String> {
        self.definitions.keys().cloned().collect()
    }
}

pub fn spawn_building(
    commands: &mut Commands,
    registry: &BuildingRegistry,
    building_name: &str,
    grid_x: i32,
    grid_y: i32,
    world_pos: Vec2,
) -> (Entity, i32) {
    registry.spawn_building(commands, building_name, grid_x, grid_y, world_pos)
        .unwrap_or_else(|| panic!("Building name '{}' not found in registry", building_name))
}

pub fn occupy_area(
    grid_cells: &mut Query<(Entity, &Position, &mut CellChildren)>,
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
    building_entity: Entity,
) {
    let half_width = width / 2;
    let half_height = height / 2;

    for dy in -half_height..=half_height {
        for dx in -half_width..=half_width {
            let check_x = center_x + dx;
            let check_y = center_y + dy;

            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == check_x && pos.y == check_y)
            {
                cell_children.0.push(building_entity);
            }
        }
    }
}

#[derive(Component)]
pub struct Hub;


pub fn place_hub(
    mut commands: Commands,
    grid: Res<Grid>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    let center_x = 0;
    let center_y = 0;

    let world_pos = grid.grid_to_world_coordinates(center_x, center_y);

    // Create central inventory with starting ore
    let mut central_inventory = Inventory::new(10000); // Large capacity for central storage
    central_inventory.add_item(create_ore_item(), 800); // Starting ore amount

    let building_entity = commands.spawn((
        Building,
        Hub,
        Name { name: "Command Hub".to_string() },
        Position { x: center_x, y: center_y },
        MultiCellBuilding { 
            width: 3, 
            height: 3, 
            center_x, 
            center_y 
        },
        PowerGenerator { amount: 100 },
        ComputeGenerator { amount: 60 },
        central_inventory, // Add the central inventory
        Operational(true),
        Layer(BUILDING_LAYER),
    ))
    .insert(Sprite::from_color(Color::srgb(0.3, 0.3, 0.7), Vec2::new(120.0, 120.0)))
    .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
    .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}
