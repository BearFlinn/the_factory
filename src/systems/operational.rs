use bevy::prelude::*;
use crate::{
    grid::Position, items::Inventory, structures::{Building, Hub, PowerConsumer, PowerGenerator, ResourceConsumer}, systems::{power, NetworkConnectivity, PowerGrid}
};

#[derive(Component)]
pub struct Operational(pub bool);

pub fn update_operational_status_optimized(
    mut buildings: Query<(&Position, &mut Operational, Option<&PowerConsumer>, Option<&ResourceConsumer>, Option<&PowerGenerator>), With<Building>>,
    network_connectivity: Res<NetworkConnectivity>,
    power_grid: Res<PowerGrid>,
    central_inventory: Query<&Inventory, With<Hub>>,
) {
    let has_power = power_grid.available >= 0;
    let inventory = central_inventory.get_single().ok();
    
    for (pos, mut operational, power_consumer, resource_consumer, power_generator) in buildings.iter_mut() {
        if !network_connectivity.is_adjacent_to_connected_network(pos.x, pos.y) {
            operational.0 = false;
            continue; 
        }
        
        // Check resource availability for resource consumers
        if let Some(consumer) = resource_consumer {
            if let Some(inv) = inventory {
                if !inv.has_item(0, consumer.amount) { // 0 is ore ID
                    operational.0 = false;
                    continue;
                }
            } else {
                operational.0 = false;
                continue;
            }
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