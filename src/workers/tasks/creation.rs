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
    workers::{manhattan_distance_coords, Worker, WorkerState},
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Splits a contribution into multiple chunks that fit within worker capacity
fn chunk_contribution_by_capacity(
    contribution: HashMap<ItemName, u32>,
    capacity: u32,
) -> Vec<HashMap<ItemName, u32>> {
    let mut chunks = Vec::new();
    let mut remaining_items = contribution;

    while !remaining_items.is_empty() {
        let mut current_chunk = HashMap::new();
        let mut current_chunk_size = 0;

        // Fill current chunk up to capacity
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

        // Remove depleted items
        for item_name in items_to_remove {
            remaining_items.remove(&item_name);
        }

        // Add completed chunk to results
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        // Safety check to prevent infinite loops
        if current_chunk_size == 0 {
            break;
        }
    }

    chunks
}

/// Creates logistics tasks from `PortLogisticsRequest` events.
/// Handles both offers (items to pick up from `OutputPort`s) and requests
/// (items needed by `InputPort`s).
///
/// The `PortLogisticsRequest` event has:
/// - `building`: The building entity requesting logistics
/// - `item`: The item type being offered or requested
/// - `quantity`: How much of the item
/// - `is_output`: If true, building is offering items for pickup; if false, needs delivery
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
) {
    let existing_targets: HashSet<Entity> = existing_tasks.iter().map(|target| target.0).collect();

    for event in events.read() {
        if existing_targets.contains(&event.building) {
            continue;
        }

        // Get the building's position
        let Ok(building_pos) = buildings_with_pos.get(event.building) else {
            continue;
        };

        // Convert single item request to HashMap for compatibility with helper functions
        let items: HashMap<ItemName, u32> =
            [(event.item.clone(), event.quantity)].into_iter().collect();

        if event.is_output {
            // This building is offering items for pickup - find a receiver
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
            // This building needs items delivered - find suppliers
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
                    Some(items_to_pickup),
                    Priority::Medium,
                );
            }
        }
    }
}

/// Finds a receiver for items being offered (`InputPort` or `StoragePort` that can accept items).
/// For `InputPort` buildings with recipes, only considers those whose recipe needs the offered items.
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

    // Check StoragePort buildings (preferred - they accept any items)
    for (entity, storage, pos) in storage_ports.iter() {
        if entity == sender || existing_targets.contains(&entity) {
            continue;
        }

        // Check network connectivity - both must be connected
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

    // Check InputPort buildings that might accept items
    for (entity, input, pos, maybe_crafter) in input_ports.iter() {
        if entity == sender || existing_targets.contains(&entity) {
            continue;
        }

        // Check network connectivity
        if !network.is_cell_connected(source_pos.x, source_pos.y)
            || !network.is_cell_connected(pos.x, pos.y)
        {
            continue;
        }

        if !input.has_space_for(items) {
            continue;
        }

        // If building has a recipe, only accept items that are recipe inputs
        if let Some(crafter) = maybe_crafter {
            if let Some(recipe_name) = crafter.get_active_recipe() {
                if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
                    // Check if any offered item is a recipe input
                    let accepts_any_item = items
                        .keys()
                        .any(|item_name| recipe_def.inputs.contains_key(item_name));
                    if !accepts_any_item {
                        continue; // Building doesn't need any of the offered items
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

/// Finds suppliers for items being requested (`OutputPort` or `StoragePort` that have items).
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

        // Check OutputPort buildings
        for (entity, output, pos) in output_ports.iter() {
            if entity == receiver || existing_targets.contains(&entity) {
                continue;
            }

            // Check network connectivity
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

        // Check StoragePort buildings
        for (entity, storage, pos) in storage_ports.iter() {
            if entity == receiver || existing_targets.contains(&entity) {
                continue;
            }

            // Check network connectivity
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

        // Process best supplier
        if let Some((entity, pos, contribution, _)) = best_supplier {
            let chunks = chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);

            for chunk in chunks {
                supply_plan.push((entity, pos, chunk.clone()));

                // Reserve items
                let reserved = reserved_items.entry(entity).or_default();
                for (item_name, amount) in &chunk {
                    *reserved.entry(item_name.clone()).or_insert(0) += amount;
                }

                // Update remaining needs
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

/// Calculates what a port-based supplier can contribute toward fulfilling needs.
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

/// Creates proactive tasks to move items from `OutputPort`s to `StoragePort`s when no immediate need.
pub fn create_proactive_port_tasks(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ProactiveTaskTimer>,
    idle_workers: Query<Entity, (With<Worker>, With<WorkerState>)>,
    output_ports: Query<(Entity, &OutputPort, &Position), With<Building>>,
    storage_ports: Query<(Entity, &StoragePort, &Position), With<Building>>,
    existing_tasks: Query<&TaskTarget, With<Task>>,
    network: Res<NetworkConnectivity>,
) {
    // Only run periodically
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }

    // Count idle workers to determine how many proactive tasks to create
    let idle_count = idle_workers.iter().count();
    if idle_count == 0 {
        return;
    }

    // Get existing task targets to avoid conflicts
    let existing_targets: HashSet<Entity> = existing_tasks.iter().map(|target| target.0).collect();

    let mut proactive_sequences = Vec::new();

    // OutputPort -> StoragePort: Move produced items to storage
    for (output_entity, output, output_pos) in output_ports.iter() {
        if existing_targets.contains(&output_entity) {
            continue;
        }

        if output.is_empty() {
            continue;
        }

        // Check network connectivity for output building
        if !network.is_cell_connected(output_pos.x, output_pos.y) {
            continue;
        }

        // Find closest connected storage with space
        let mut best_storage: Option<(Entity, Position, i32)> = None;
        let output_coords = (output_pos.x, output_pos.y);

        for (storage_entity, storage, storage_pos) in storage_ports.iter() {
            if existing_targets.contains(&storage_entity) {
                continue;
            }

            // Check network connectivity for storage building
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
            // Calculate items to transfer (respecting worker capacity)
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

    // Create the task sequences
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

/// Calculate feasible transfer from a port up to worker capacity.
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

/// Port-based construction logistics: queries `StoragePort` for construction materials.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn create_port_construction_logistics_tasks(
    mut commands: Commands,
    mut construction_requests: EventReader<ConstructionMaterialRequest>,
    storage_ports: Query<(Entity, &StoragePort, &Position), With<Building>>,
    network: Res<NetworkConnectivity>,
) {
    for request in construction_requests.read() {
        let requester_pos = (request.position.x, request.position.y);

        // Check if construction site is connected
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
            // Create separate task sequences for each supplier to enable parallel work
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

                // Create individual sequence for each supplier (enables parallel work)
                let sequence_entity = commands
                    .spawn(TaskSequenceBundle::new(
                        vec![pickup_task, dropoff_task],
                        request.priority.clone(),
                    ))
                    .id();

                // Link tasks to their sequence
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

/// Calculate supply plan from `StoragePort` buildings for construction materials.
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
            // Check network connectivity
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

            // Prefer suppliers that can provide substantial amounts
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

        // Process best supplier with capacity chunking
        if let Some((entity, pos, contribution, _)) = best_supplier {
            let chunked_contributions =
                chunk_contribution_by_capacity(contribution, WORKER_CAPACITY);

            for chunk in chunked_contributions {
                supply_plan.push((entity, pos, chunk.clone()));

                // Reserve items from this supplier
                let reserved_for_entity = reserved_items.entry(entity).or_default();
                for (item_name, contributed_amount) in &chunk {
                    *reserved_for_entity.entry(item_name.clone()).or_insert(0) +=
                        contributed_amount;
                }

                // Subtract this chunk from remaining needs
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

/// Creates a pickup-dropoff task sequence for moving items between buildings.
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
