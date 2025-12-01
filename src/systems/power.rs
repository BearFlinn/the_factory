use bevy::prelude::*;
use crate::{
    structures::{PowerGenerator, PowerConsumer},
    systems::{Operational}
};

#[derive(Resource, Default)]
pub struct PowerGrid {
    pub capacity: i32,
    pub usage: i32,
    pub available: i32
}

pub fn update_power_grid(
    mut power_grid: ResMut<PowerGrid>,
    generators: Query<(&PowerGenerator, &Operational)>,
    consumers: Query<&PowerConsumer>,
) {
    let mut total_production: i32 = 0;
    for (generator, operational) in generators.iter() {
        if operational.get_status() == false {
            continue;
        }
        
        total_production += generator.amount;
        
    }

    let total_consumption: i32 = consumers.iter().map(|c| c.amount).sum();

    power_grid.capacity = total_production;
    power_grid.usage = total_consumption;
    power_grid.available = total_production - total_consumption;
}