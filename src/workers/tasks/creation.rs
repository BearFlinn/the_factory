use super::components::{
    Priority, SequenceMember, Task, TaskAction, TaskBundle, TaskSequenceBundle, TaskTarget,
};
use crate::{
    grid::Position,
    materials::{
        items::{InputPort, InventoryAccess, OutputPort, StoragePort},
        ItemName, RecipeRegistry,
    },
    structures::{Building, ConstructionMaterialRequest, PortLogisticsRequest, RecipeCrafter},
    systems::NetworkConnectivity,
    workers::{manhattan_distance_coords, Worker},
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

fn chunk_contribution_by_capacity(
    contribution: HashMap<ItemName, u32>,
    capacity: u32,
) -> Vec<HashMap<ItemName, u32>> {
    let mut chunks = Vec::new();
    let mut remaining_items = contribution;

    while !remaining_items.is_empty() {
        let mut current_chunk = HashMap::new();
        let mut current_chunk_size = 0;
        let mut items_to_remove = Vec::new();

        for (item_name, quantity) in &mut remaining_items {
            if current_chunk_size >= capacity {
                break;
            }

            let available_space = capacity - current_chunk_size;
            let items_to_take = (*quantity).min(available_space);

            if items_to_take > 0 {
                current_chunk.insert(item_name.clone(), items_to_take);
                current_chunk_size += items_to_take;
                *quantity -= items_to_take;

                if *quantity == 0 {
                    items_to_remove.push(item_name.clone());
                }
            }
        }

        for item_name in items_to_remove {
            remaining_items.remove(&item_name);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        if current_chunk_size == 0 {
            break;
        }
    }

    chunks
}

pub fn create_port_logistics_tasks(
    mut commands: Commands,
    mut events: EventReader<PortLogisticsRequest>,
    buildings_with_pos: Query<&Position, With<Building>>,
    output_ports: Query<(Entity, &OutputPort, &Position), With<Building>>,
    input_ports: Query<(Entity, &InputPort, &Position, Option<&RecipeCrafter>), With<Building>>,
    storage_ports: Query<(Entity, &StoragePort, &Position), With<Building>>,
    existing_tasks: Query<&TaskTarget, With<Task>>,
    network: Res<NetworkConnectivity>,
    recipe_registry: Res<RecipeRegistry>,
    mut delivery_started_events: EventWriter<super::components::LogisticsDeliveryStartedEvent>,
) {
    let existing_targets: HashSet<Entity> = existing_tasks.iter().map(|target| target.0).collect();

    for event in events.read() {
        if existing_targets.contains(&event.building) {
            continue;
        }

        let Ok(building_pos) = buildings_with_pos.get(event.building) else {
            continue;
        };

        let items: HashMap<ItemName, u32> =
            [(event.item.clone(), event.quantity)].into_iter().collect();

        if event.is_output {
            let receiver = find_port_receiver(
                &items,
                *building_pos,
                event.building,
                &input_ports,
                &storage_ports,
                &existing_targets,
                &network,
                &recipe_registry,
            );

            if let Some((receiver_entity, receiver_pos)) = receiver {
                create_pickup_dropoff_sequence(
                    &mut commands,
                    event.building,
                    *building_pos,
                    receiver_entity,
                    receiver_pos,
                    Some(items),
                    Priority::Medium,
                );
            }
        } else {
            let supply_plan = find_port_suppliers(
                &items,
                *building_pos,
                event.building,
                &output_ports,
                &storage_ports,
                &existing_targets,
                &network,
            );

            for (supplier_entity, supplier_pos, items_to_pickup) in supply_plan {
                create_pickup_dropoff_sequence(
                    &mut commands,
                    supplier_entity,
                    supplier_pos,
                    event.building,
                    *building_pos,
                    Some(items_to_pickup.clone()),
                    Priority::Medium,
                );

                delivery_started_events.send(super::components::LogisticsDeliveryStartedEvent {
                    building: event.building,
                    items: items_to_pickup,
                });
            }
        }
    }
}

fn find_port_receiver(
    items: &HashMap<ItemName, u32>,
    source_pos: Position,
    sender: Entity,
    input_ports: &Query<(Entity, &InputPort, &Position, Option<&RecipeCrafter>), With<Building>>,
    storage_ports: &Query<(Entity, &StoragePort, &Position), With<Building>>,
    existing_targets: &HashSet<Entity>,
    network: &NetworkConnectivity,
    recipe_registry: &RecipeRegistry,
) -> Option<(Entity, Position)> {
    let mut best_receiver: Option<(Entity, Position, i32)> = None;
    let source_coords = (source_pos.x, source_pos.y);

    for (entity, storage, pos) in storage_ports.iter() {
        if entity == sender || existing_targets.contains(&entity) {
            continue;
        }

        if !network.is_cell_connected(source_pos.x, source_pos.y)
            || !network.is_cell_connected(pos.x, pos.y)
        {
            continue;
        }

        if !storage.has_space_for(items) {
            continue;
        }

        let distance = manhattan_distance_coords(source_coords, (pos.x, pos.y));
        if best_receiver.is_none_or(|(_, _, d)| distance < d) {
            best_receiver = Some((entity, *pos, distance));
        }
    }

    for (entity, input, pos, maybe_crafter) in input_ports.iter() {
        if entity == sender || existing_targets.contains(&entity) {
            continue;
        }

        if !network.is_cell_connected(source_pos.x, source_pos.y)
            || !network.is_cell_connected(pos.x, pos.y)
        {
            continue;
        }

        if !input.has_space_for(items) {
            continue;
        }

        if let Some(crafter) = maybe_crafter {
            if let Some(recipe_name) = crafter.get_active_recipe() {
                if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                    let accepts_any_item = items
                        .keys()
                        .any(|item_name| recipe_def.inputs.contains_key(item_name));
                    if !accepts_any_item {
                        continue;
                    }
                }
            }
        }

        let distance = manhattan_distance_coords(source_coords, (pos.x, pos.y));
        if best_receiver.is_none_or(|(_, _, d)| distance < d) {
            best_receiver = Some((entity, *pos, distance));
        }
    }

    best_receiver.map(|(e, p, _)| (e, p))
}

fn find_port_suppliers(
    needed_items: &HashMap<ItemName, u32>,
    requester_pos: Position,
    receiver: Entity,
    output_ports: &Query<(Entity, &OutputPort, &Position), With<Building>>,
    storage_ports: &Query<(Entity, &StoragePort, &Position), With<Building>>,
    existing_targets: &HashSet<Entity>,
    network: &NetworkConnectivity,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    const WORKER_CAPACITY: u32 = 20;
    let requester_coords = (requester_pos.x, requester_pos.y);

    let mut remaining_needs = needed_items.clone();
    let mut supply_plan = Vec::new();
    let mut reserved_items: HashMap<Entity, HashMap<ItemName, u32>> = HashMap::new();

    while !remaining_needs.is_empty() {
        let mut best_supplier: Option<(Entity, Position, HashMap<ItemName, u32>, f32)> = None;

        for (entity, output, pos) in output_ports.iter() {
            if entity == receiver || existing_targets.contains(&entity) {
                continue;
            }

            if !network.is_cell_connected(requester_pos.x, requester_pos.y)
                || !network.is_cell_connected(pos.x, pos.y)
            {
                continue;
            }

            let contribution = calculate_port_supplier_contribution(
                entity,
                output,
                &remaining_needs,
                &reserved_items,
            );
            if contribution.is_empty() {
                continue;
            }

            let total_value: u32 = contribution.values().sum();
            let distance = manhattan_distance_coords(requester_coords, (pos.x, pos.y));
            #[allow(clippy::cast_precision_loss)]
            let score = total_value as f32 / (distance as f32 + 1.0);

            let is_better = best_supplier.as_ref().is_none_or(|(_, _, _, s)| score > *s);
            if is_better {
                best_supplier = Some((entity, *pos, contribution, score));
            }
        }

        for (entity, storage, pos) in storage_ports.iter() {
            if entity == receiver || existing_targets.contains(&entity) {
                continue;
            }

            if !network.is_cell_connected(requester_pos.x, requester_pos.y)
                || !network.is_cell_connected(pos.x, pos.y)
            {
                continue;
            }

            let contribution = calculate_port_supplier_contribution(
                entity,
                storage,
                &remaining_needs,
                &reserved_items,
            );
            if contribution.is_empty() {
                continue;
            }

            let total_value: u32 = contribution.values().sum();
            let distance = manhattan_distance_coords(requester_coords, (pos.x, pos.y));
            #[allow(clippy::cast_precision_loss)]
            let score = total_value as f32 / (distance as f32 + 1.0);

            let is_better = best_supplier.as_ref().is_none_or(|(_, _, _, s)| score > *s);
            if is_better {
                best_supplier = Some((entity, *pos, contribution, score));
            }
        }

        if let Some((entity, pos, contribution, _)) = best_supplier {
            let chunks = chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);

            for chunk in chunks {
                supply_plan.push((entity, pos, chunk.clone()));

                let reserved = reserved_items.entry(entity).or_default();
                for (item_name, amount) in &chunk {
                    *reserved.entry(item_name.clone()).or_insert(0) += amount;
                }

                for (item_name, amount) in &chunk {
                    if let Some(still_needed) = remaining_needs.get_mut(item_name) {
                        *still_needed = still_needed.saturating_sub(*amount);
                        if *still_needed == 0 {
                            remaining_needs.remove(item_name);
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    supply_plan
}

fn calculate_port_supplier_contribution<T: InventoryAccess>(
    entity: Entity,
    port: &T,
    needs: &HashMap<ItemName, u32>,
    reserved: &HashMap<Entity, HashMap<ItemName, u32>>,
) -> HashMap<ItemName, u32> {
    let mut contribution = HashMap::new();
    let reserved_for_entity = reserved.get(&entity).cloned().unwrap_or_default();

    for (item_name, &needed) in needs {
        let available = port.get_item_quantity(item_name);
        let already_reserved = reserved_for_entity.get(item_name).copied().unwrap_or(0);
        let actual_available = available.saturating_sub(already_reserved);

        if actual_available > 0 {
            contribution.insert(item_name.clone(), actual_available.min(needed));
        }
    }

    contribution
}

pub fn create_proactive_port_tasks(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ProactiveTaskTimer>,
    idle_workers: Query<Entity, With<Worker>>,
    output_ports: Query<(Entity, &OutputPort, &Position), With<Building>>,
    storage_ports: Query<(Entity, &StoragePort, &Position), With<Building>>,
    existing_tasks: Query<&TaskTarget, With<Task>>,
    network: Res<NetworkConnectivity>,
) {
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }

    let idle_count = idle_workers.iter().count();
    if idle_count == 0 {
        return;
    }

    let existing_targets: HashSet<Entity> = existing_tasks.iter().map(|target| target.0).collect();

    let mut proactive_sequences = Vec::new();

    for (output_entity, output, output_pos) in output_ports.iter() {
        if existing_targets.contains(&output_entity) {
            continue;
        }

        if output.is_empty() {
            continue;
        }

        if !network.is_cell_connected(output_pos.x, output_pos.y) {
            continue;
        }

        let mut best_storage: Option<(Entity, Position, i32)> = None;
        let output_coords = (output_pos.x, output_pos.y);

        for (storage_entity, storage, storage_pos) in storage_ports.iter() {
            if existing_targets.contains(&storage_entity) {
                continue;
            }

            if !network.is_cell_connected(storage_pos.x, storage_pos.y) {
                continue;
            }

            if storage.is_full() {
                continue;
            }

            let distance = manhattan_distance_coords(output_coords, (storage_pos.x, storage_pos.y));
            if best_storage.is_none_or(|(_, _, d)| distance < d) {
                best_storage = Some((storage_entity, *storage_pos, distance));
            }
        }

        if let Some((storage_entity, storage_pos, _)) = best_storage {
            let items_to_move = calculate_port_feasible_transfer(output, 20);
            if !items_to_move.is_empty() {
                proactive_sequences.push((
                    output_entity,
                    *output_pos,
                    storage_entity,
                    storage_pos,
                    items_to_move,
                ));
            }
        }

        if proactive_sequences.len() >= idle_count {
            break;
        }
    }

    for (pickup_entity, pickup_pos, dropoff_entity, dropoff_pos, items) in
        proactive_sequences.into_iter().take(idle_count)
    {
        let pickup_task = commands
            .spawn(TaskBundle::new(
                pickup_entity,
                pickup_pos,
                TaskAction::Pickup(Some(items.clone())),
                Priority::Low,
            ))
            .id();

        let dropoff_task = commands
            .spawn(TaskBundle::new(
                dropoff_entity,
                dropoff_pos,
                TaskAction::Dropoff(Some(items)),
                Priority::Low,
            ))
            .id();

        let sequence_entity = commands
            .spawn(TaskSequenceBundle::new(
                vec![pickup_task, dropoff_task],
                Priority::Low,
            ))
            .id();

        commands
            .entity(pickup_task)
            .insert(SequenceMember(sequence_entity));
        commands
            .entity(dropoff_task)
            .insert(SequenceMember(sequence_entity));
    }
}

fn calculate_port_feasible_transfer<T: InventoryAccess>(
    source: &T,
    worker_capacity: u32,
) -> HashMap<ItemName, u32> {
    let mut transfer = HashMap::new();
    let mut total_transfer = 0;

    for (item_name, &quantity) in source.items() {
        if total_transfer >= worker_capacity {
            break;
        }

        let transfer_amount = quantity.min(worker_capacity - total_transfer);
        if transfer_amount > 0 {
            transfer.insert(item_name.clone(), transfer_amount);
            total_transfer += transfer_amount;
        }
    }

    transfer
}

#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn create_port_construction_logistics_tasks(
    mut commands: Commands,
    mut construction_requests: EventReader<ConstructionMaterialRequest>,
    storage_ports: Query<(Entity, &StoragePort, &Position), With<Building>>,
    network: Res<NetworkConnectivity>,
) {
    for request in construction_requests.read() {
        let requester_pos = (request.position.x, request.position.y);

        if !network.is_cell_connected(request.position.x, request.position.y) {
            continue;
        }

        let supply_plan = calculate_port_supply_plan(
            requester_pos,
            &request.needed_materials,
            &storage_ports,
            &network,
        );

        if !supply_plan.is_empty() {
            for (supplier_entity, supplier_pos, items_to_pickup) in supply_plan {
                let pickup_task = commands
                    .spawn(TaskBundle::new(
                        supplier_entity,
                        supplier_pos,
                        TaskAction::Pickup(Some(items_to_pickup.clone())),
                        request.priority.clone(),
                    ))
                    .id();

                let dropoff_task = commands
                    .spawn(TaskBundle::new(
                        request.site,
                        request.position,
                        TaskAction::Dropoff(Some(items_to_pickup)),
                        request.priority.clone(),
                    ))
                    .id();

                let sequence_entity = commands
                    .spawn(TaskSequenceBundle::new(
                        vec![pickup_task, dropoff_task],
                        request.priority.clone(),
                    ))
                    .id();

                commands
                    .entity(pickup_task)
                    .insert(SequenceMember(sequence_entity));
                commands
                    .entity(dropoff_task)
                    .insert(SequenceMember(sequence_entity));
            }
        }
    }
}

fn calculate_port_supply_plan(
    requester_pos: (i32, i32),
    needed_items: &HashMap<ItemName, u32>,
    storage_ports: &Query<(Entity, &StoragePort, &Position), With<Building>>,
    network: &NetworkConnectivity,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    const WORKER_CAPACITY: u32 = 20;

    let mut remaining_needs = needed_items.clone();
    let mut supply_plan = Vec::new();
    let mut reserved_items: HashMap<Entity, HashMap<ItemName, u32>> = HashMap::new();

    while !remaining_needs.is_empty() {
        let mut best_supplier: Option<(Entity, Position, HashMap<ItemName, u32>, f32)> = None;

        for (entity, storage, pos) in storage_ports.iter() {
            if !network.is_cell_connected(pos.x, pos.y) {
                continue;
            }

            let contribution = calculate_port_supplier_contribution(
                entity,
                storage,
                &remaining_needs,
                &reserved_items,
            );
            if contribution.is_empty() {
                continue;
            }

            let total_value: u32 = contribution.values().sum();
            let distance = manhattan_distance_coords(requester_pos, (pos.x, pos.y));
            #[allow(clippy::cast_precision_loss)]
            let score = total_value as f32 / (distance as f32 + 1.0);

            let substantial_bonus = if total_value >= WORKER_CAPACITY {
                2.0
            } else {
                1.0
            };
            let final_score = score * substantial_bonus;

            let is_better = best_supplier
                .as_ref()
                .is_none_or(|(_, _, _, s)| final_score > *s);
            if is_better {
                best_supplier = Some((entity, *pos, contribution, final_score));
            }
        }

        if let Some((entity, pos, contribution, _)) = best_supplier {
            let chunked_contributions =
                chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);

            for chunk in chunked_contributions {
                supply_plan.push((entity, pos, chunk.clone()));

                let reserved_for_entity = reserved_items.entry(entity).or_default();
                for (item_name, contributed_amount) in &chunk {
                    *reserved_for_entity.entry(item_name.clone()).or_insert(0) +=
                        contributed_amount;
                }

                for (item_name, contributed_amount) in &chunk {
                    if let Some(still_needed) = remaining_needs.get_mut(item_name) {
                        *still_needed = still_needed.saturating_sub(*contributed_amount);
                        if *still_needed == 0 {
                            remaining_needs.remove(item_name);
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    supply_plan
}

#[derive(Resource)]
pub struct ProactiveTaskTimer {
    timer: Timer,
}

impl Default for ProactiveTaskTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        }
    }
}

fn create_pickup_dropoff_sequence(
    commands: &mut Commands,
    pickup_entity: Entity,
    pickup_pos: Position,
    dropoff_entity: Entity,
    dropoff_pos: Position,
    items: Option<HashMap<ItemName, u32>>,
    priority: Priority,
) {
    let pickup_task = commands
        .spawn(TaskBundle::new(
            pickup_entity,
            pickup_pos,
            TaskAction::Pickup(items.clone()),
            priority.clone(),
        ))
        .id();

    let dropoff_task = commands
        .spawn(TaskBundle::new(
            dropoff_entity,
            dropoff_pos,
            TaskAction::Dropoff(items),
            priority.clone(),
        ))
        .id();

    let sequence_entity = commands
        .spawn(TaskSequenceBundle::new(
            vec![pickup_task, dropoff_task],
            priority,
        ))
        .id();

    commands
        .entity(pickup_task)
        .insert(SequenceMember(sequence_entity));
    commands
        .entity(dropoff_task)
        .insert(SequenceMember(sequence_entity));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn chunk_contribution_under_capacity_single_chunk() {
        let mut contribution = HashMap::new();
        contribution.insert("iron".to_string(), 5);
        contribution.insert("copper".to_string(), 10);

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        assert_eq!(chunks.len(), 1);
        let total: u32 = chunks[0].values().sum();
        assert_eq!(total, 15);
    }

    #[test]
    fn chunk_contribution_at_capacity_single_chunk() {
        let mut contribution = HashMap::new();
        contribution.insert("iron".to_string(), 10);
        contribution.insert("copper".to_string(), 10);

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        assert_eq!(chunks.len(), 1);
        let total: u32 = chunks[0].values().sum();
        assert_eq!(total, 20);
    }

    #[test]
    fn chunk_contribution_over_capacity_multiple_chunks() {
        let mut contribution = HashMap::new();
        contribution.insert("iron".to_string(), 30);

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].get("iron"), Some(&20));
        assert_eq!(chunks[1].get("iron"), Some(&10));
    }

    #[test]
    fn chunk_contribution_multiple_items_over_capacity() {
        let mut contribution = HashMap::new();
        contribution.insert("iron".to_string(), 25);
        contribution.insert("copper".to_string(), 15);

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        // Should create multiple chunks
        assert!(chunks.len() >= 2);

        // Total should equal original contribution
        let total: u32 = chunks.iter().flat_map(|c| c.values()).sum();
        assert_eq!(total, 40);

        // Each chunk should respect capacity
        for chunk in &chunks {
            let chunk_total: u32 = chunk.values().sum();
            assert!(chunk_total <= 20);
        }
    }

    #[test]
    fn chunk_contribution_empty_contribution() {
        let contribution = HashMap::new();

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_contribution_single_item_exact_capacity() {
        let mut contribution = HashMap::new();
        contribution.insert("iron".to_string(), 20);

        let chunks = chunk_contribution_by_capacity(contribution, 20);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].get("iron"), Some(&20));
    }

    #[test]
    fn chunk_contribution_many_small_items() {
        let mut contribution = HashMap::new();
        contribution.insert("a".to_string(), 5);
        contribution.insert("b".to_string(), 5);
        contribution.insert("c".to_string(), 5);
        contribution.insert("d".to_string(), 5);
        contribution.insert("e".to_string(), 5);

        let chunks = chunk_contribution_by_capacity(contribution, 10);

        // Total should equal 25
        let total: u32 = chunks.iter().flat_map(|c| c.values()).sum();
        assert_eq!(total, 25);

        // Should need at least 3 chunks for 25 items with capacity 10
        assert!(chunks.len() >= 3);

        // Each chunk should respect capacity
        for chunk in &chunks {
            let chunk_total: u32 = chunk.values().sum();
            assert!(chunk_total <= 10);
        }
    }
}
