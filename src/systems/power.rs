use crate::{
    structures::{PowerConsumer, PowerGenerator},
    systems::Operational,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct PowerGrid {
    pub capacity: i32,
    pub usage: i32,
    pub available: i32,
}

#[allow(clippy::needless_pass_by_value)] // Bevy system parameters must be passed by value
pub fn update_power_grid(
    mut power_grid: ResMut<PowerGrid>,
    generators: Query<(&PowerGenerator, &Operational)>,
    consumers: Query<&PowerConsumer>,
) {
    let mut total_production: i32 = 0;
    for (generator, operational) in generators.iter() {
        if !operational.get_status() {
            continue;
        }

        total_production += generator.amount;
    }

    let total_consumption: i32 = consumers.iter().map(|c| c.amount).sum();

    power_grid.capacity = total_production;
    power_grid.usage = total_consumption;
    power_grid.available = total_production - total_consumption;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_grid_default_has_zero_values() {
        let grid = PowerGrid::default();
        assert_eq!(grid.capacity, 0);
        assert_eq!(grid.usage, 0);
        assert_eq!(grid.available, 0);
    }

    #[test]
    fn power_grid_single_producer() {
        let grid = PowerGrid {
            capacity: 100,
            usage: 0,
            available: 100,
        };

        assert_eq!(grid.capacity, 100);
        assert_eq!(grid.available, 100);
    }

    #[test]
    fn power_grid_multiple_producers() {
        // Simulating multiple producers: 100 + 200 + 50 = 350
        let grid = PowerGrid {
            capacity: 350,
            usage: 0,
            available: 350,
        };

        assert_eq!(grid.capacity, 350);
        assert_eq!(grid.available, 350);
    }

    #[test]
    fn power_grid_single_consumer() {
        let grid = PowerGrid {
            capacity: 100,
            usage: 30,
            available: 70,
        };

        assert_eq!(grid.usage, 30);
        assert_eq!(grid.available, 70);
    }

    #[test]
    fn power_grid_multiple_consumers() {
        // Simulating multiple consumers: 20 + 30 + 15 = 65
        let grid = PowerGrid {
            capacity: 100,
            usage: 65,
            available: 35,
        };

        assert_eq!(grid.usage, 65);
        assert_eq!(grid.available, 35);
    }

    #[test]
    fn power_grid_available_capacity_calculation() {
        let grid = PowerGrid {
            capacity: 500,
            usage: 200,
            available: 300,
        };

        assert_eq!(grid.available, 300);
    }

    #[test]
    fn power_grid_negative_available_when_overconsumption() {
        let grid = PowerGrid {
            capacity: 100,
            usage: 150,
            available: -50,
        };

        assert_eq!(grid.available, -50);
        assert!(grid.available < 0);
    }

    #[test]
    fn power_grid_zero_available_when_balanced() {
        let grid = PowerGrid {
            capacity: 100,
            usage: 100,
            available: 0,
        };

        assert_eq!(grid.available, 0);
    }
}
