use bevy::prelude::*;
use crate::{
    grid::Position, 
    materials::items::Inventory, 
    structures::{Building, PowerConsumer, PowerGenerator, ResourceConsumer, IRON_ORE}, 
    systems::{NetworkConnectivity, PowerGrid}
};

#[derive(Component)]
pub struct Operational(pub bool);

pub fn update_consumer_operation(
    mut consumer_buildings: Query<(&mut Operational, &ResourceConsumer, &Inventory), With<Building>>,
) {
    for (mut operational, consumer, inventory) in consumer_buildings.iter_mut() {
        operational.0 = inventory.has_item(IRON_ORE, consumer.amount); // 0 is ore ID
    }
}

pub fn update_operational_status_optimized(
    mut buildings: Query<(&Position, &mut Operational, Option<&PowerConsumer>, Option<&PowerGenerator>), (With<Building>, Without<ResourceConsumer>)>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
) {
    let has_power = power_grid.available >= 0;
   
    for (pos, mut operational, power_consumer, power_generator) in buildings.iter_mut() {
        if !network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y) {
            operational.0 = false;
            continue;
        }
       
        operational.0 = if power_generator.is_some() {
            true
        } else {
            if power_consumer.is_some() {
                has_power
            } else {
                true
            }
        }
    }
}