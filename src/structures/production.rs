use bevy::prelude::*;
use crate::{
    structures::{Hub, ResourceConsumer, Producer},
    items::{Inventory, create_ore_item},
    systems::Operational
};

pub fn update_producers(
    mut query: Query<(&mut Producer, &Operational, &mut Inventory)>,
    time: Res<Time>,
) {
    for (mut producer, operational, mut inventory) in query.iter_mut() {
        if !operational.0 {
            continue;
        }
        
        if producer.timer.tick(time.delta()).just_finished() {
            inventory.add_item(create_ore_item(), producer.amount);
            producer.timer.reset();
        }
    }
}

pub fn update_resource_consumers(
    mut query: Query<(&mut ResourceConsumer, &Operational)>,
    mut central_inventory: Query<&mut Inventory, With<Hub>>,
    time: Res<Time>,
) {
    let Ok(mut inventory) = central_inventory.get_single_mut() else {
        return; // No central hub found
    };

    for (mut consumer, operational) in query.iter_mut() {
        if !operational.0 {
            continue;
        }
        
        if consumer.timer.tick(time.delta()).just_finished() {
            if inventory.has_item(0, consumer.amount) { // 0 is ore ID
                inventory.remove_item(0, consumer.amount);
                consumer.timer.reset();
            }
            // Note: If insufficient resources, timer continues but no consumption occurs
            // This allows the building to resume when resources become available
        }
    }
}