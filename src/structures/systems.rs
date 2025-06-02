use bevy::prelude::*;

use super::{PowerConsumer, PowerGenerator, Producer};

#[derive(Resource, Default)]
pub struct TotalProduction {
    pub ore: u32,
}

#[derive(Resource, Default)]
pub struct PowerGrid {
    pub capacity: i32,
    pub usage: i32,
    pub available: i32
}

pub fn update_power_grid(
    mut power_grid: ResMut<PowerGrid>,
    generators: Query<&PowerGenerator>,
    consumers: Query<&PowerConsumer>,
) {
    let total_production: i32 = generators.iter().map(|g| g.amount).sum();
    let total_consumption: i32 = consumers.iter().map(|c| c.amount).sum();

    power_grid.capacity = total_production;
    power_grid.usage = total_consumption;
    power_grid.available = total_production - total_consumption;
}

pub fn update_producers(
    mut query: Query<&mut Producer>,
    mut total_production: ResMut<TotalProduction>,
    time: Res<Time>,
) {
    for mut producer in query.iter_mut() {
        if producer.timer.tick(time.delta()).just_finished() {
            total_production.ore += producer.amount;
            producer.timer.reset();
        }
    }
}