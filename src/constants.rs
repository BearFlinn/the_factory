#![allow(dead_code)]

use crate::materials::items::ItemId;
use crate::structures::BuildingId;

pub mod items {
    use super::ItemId;
    
    pub const IRON_ORE: ItemId = 0;
    pub const COPPER_ORE: ItemId = 1;
    pub const COAL: ItemId = 2;
}

pub mod structures {
    use super::BuildingId;

    pub const HUB: BuildingId = 0;
    pub const MINING_DRILL: BuildingId = 1;
    pub const CONNECTOR: BuildingId = 2;
    pub const RADAR: BuildingId = 3;
    pub const GENERATOR: BuildingId = 4;
    pub const DATACENTER: BuildingId = 5;
}


pub mod gridlayers {
    pub const RESOURCE_LAYER: i32 = 0;
    pub const BUILDING_LAYER: i32 = 1;
}