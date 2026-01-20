use crate::{
    structures::{ComputeConsumer, ComputeGenerator},
    systems::Operational,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct ComputeGrid {
    pub capacity: i32,
    pub usage: i32,
    pub available: i32,
}

pub fn update_compute(
    mut compute_grid: ResMut<ComputeGrid>,
    generators: Query<(&ComputeGenerator, &Operational)>,
    consumers: Query<&ComputeConsumer>,
) {
    let mut total_compute: i32 = 0;
    for (generator, operational) in generators.iter() {
        if !operational.get_status() {
            continue;
        }

        total_compute += generator.amount;
    }

    let total_consumption: i32 = consumers.iter().map(|c| c.amount).sum();

    compute_grid.capacity = total_compute;
    compute_grid.usage = total_consumption;
    compute_grid.available = total_compute - total_consumption;
}
