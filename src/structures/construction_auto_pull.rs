use crate::{
    grid::Position,
    materials::{InputPort, InventoryAccess, ItemName, ItemTransferRequestEvent, StoragePort},
    structures::{BuildingCost, ConstructionSite},
    systems::NetworkConnectivity,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct ConstructionAutoPullTimer {
    pub timer: Timer,
}

impl Default for ConstructionAutoPullTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

fn compute_deficit(needed: &HashMap<ItemName, u32>, current: &InputPort) -> HashMap<ItemName, u32> {
    let mut deficit = HashMap::new();
    for (item_name, &required) in needed {
        let on_hand = current.get_item_quantity(item_name);
        if on_hand < required {
            deficit.insert(item_name.clone(), required - on_hand);
        }
    }
    deficit
}

pub fn auto_pull_construction_materials(
    time: Res<Time>,
    mut timer: ResMut<ConstructionAutoPullTimer>,
    construction_sites: Query<
        (Entity, &InputPort, &BuildingCost, &Position),
        With<ConstructionSite>,
    >,
    storage_ports: Query<(Entity, &StoragePort, &Position)>,
    network: Res<NetworkConnectivity>,
    mut transfer_events: MessageWriter<ItemTransferRequestEvent>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    for (site_entity, input_port, building_cost, site_pos) in &construction_sites {
        let deficit = compute_deficit(&building_cost.cost.inputs, input_port);
        if deficit.is_empty() {
            continue;
        }

        if !network.is_cell_connected(site_pos.x, site_pos.y) {
            continue;
        }

        let mut remaining_deficit = deficit;

        for (storage_entity, storage_port, storage_pos) in &storage_ports {
            if remaining_deficit.is_empty() {
                break;
            }

            if !network.is_cell_connected(storage_pos.x, storage_pos.y) {
                continue;
            }

            let mut transfer_items: HashMap<ItemName, u32> = HashMap::new();

            for (item_name, deficit_amount) in &remaining_deficit {
                let available = storage_port.get_item_quantity(item_name);
                if available == 0 {
                    continue;
                }
                let to_transfer = (*deficit_amount).min(available);
                transfer_items.insert(item_name.clone(), to_transfer);
            }

            if transfer_items.is_empty() {
                continue;
            }

            for (item_name, transferred) in &transfer_items {
                if let Some(remaining) = remaining_deficit.get_mut(item_name) {
                    *remaining = remaining.saturating_sub(*transferred);
                }
            }
            remaining_deficit.retain(|_, v| *v > 0);

            transfer_events.write(ItemTransferRequestEvent {
                sender: storage_entity,
                receiver: site_entity,
                items: transfer_items,
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn timer_defaults_to_one_second_repeating() {
        let auto_pull = ConstructionAutoPullTimer::default();
        assert!((auto_pull.timer.duration().as_secs_f32() - 1.0).abs() < f32::EPSILON);
        assert_eq!(auto_pull.timer.mode(), TimerMode::Repeating);
    }

    #[test]
    fn compute_deficit_full_deficit() {
        let mut needed = HashMap::new();
        needed.insert("Iron Ore".to_string(), 10);
        needed.insert("Copper Ore".to_string(), 5);

        let input_port = InputPort::new(1000);

        let deficit = compute_deficit(&needed, &input_port);

        assert_eq!(deficit.get("Iron Ore"), Some(&10));
        assert_eq!(deficit.get("Copper Ore"), Some(&5));
    }

    #[test]
    fn compute_deficit_partial_deficit() {
        let mut needed = HashMap::new();
        needed.insert("Iron Ore".to_string(), 10);
        needed.insert("Copper Ore".to_string(), 5);

        let mut input_port = InputPort::new(1000);
        input_port.add_item("Iron Ore", 3);
        input_port.add_item("Copper Ore", 5);

        let deficit = compute_deficit(&needed, &input_port);

        assert_eq!(deficit.get("Iron Ore"), Some(&7));
        assert!(!deficit.contains_key("Copper Ore"));
    }

    #[test]
    fn compute_deficit_no_deficit() {
        let mut needed = HashMap::new();
        needed.insert("Iron Ore".to_string(), 10);
        needed.insert("Copper Ore".to_string(), 5);

        let mut input_port = InputPort::new(1000);
        input_port.add_item("Iron Ore", 10);
        input_port.add_item("Copper Ore", 5);

        let deficit = compute_deficit(&needed, &input_port);

        assert!(deficit.is_empty());
    }

    #[test]
    fn compute_deficit_extra_materials() {
        let mut needed = HashMap::new();
        needed.insert("Iron Ore".to_string(), 10);

        let mut input_port = InputPort::new(1000);
        input_port.add_item("Iron Ore", 20);

        let deficit = compute_deficit(&needed, &input_port);

        assert!(deficit.is_empty());
    }

    #[test]
    fn compute_deficit_no_materials_needed() {
        let needed: HashMap<ItemName, u32> = HashMap::new();
        let input_port = InputPort::new(1000);

        let deficit = compute_deficit(&needed, &input_port);

        assert!(deficit.is_empty());
    }
}
