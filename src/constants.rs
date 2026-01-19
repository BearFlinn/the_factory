#![allow(dead_code)]

pub mod items {
    pub const IRON_ORE: &str = "Iron Ore";
    pub const COPPER_ORE: &str = "Copper Ore";
    pub const COAL: &str = "Coal";
}

pub mod structures {
    pub const HUB: &str = "Hub";
    pub const MINING_DRILL: &str = "Mining Drill";
    pub const CONNECTOR: &str = "Connector";
    pub const RADAR: &str = "Radar";
    pub const GENERATOR: &str = "Generator";
    pub const DATACENTER: &str = "Datacenter";
}

pub mod gridlayers {
    pub const RESOURCE_LAYER: i32 = 0;
    pub const BUILDING_LAYER: i32 = 1;
}
