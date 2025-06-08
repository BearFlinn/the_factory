use std::collections::{HashSet, VecDeque, HashMap};
use bevy::prelude::*;

use crate::{grid::{Grid, Position}, materials::{Inventory, InventoryType, InventoryTypes, ItemName, ItemTransferRequestEvent, RecipeRegistry}, structures::{Building, Hub, RecipeCrafter, WorkersEnRoute}, systems::NetworkConnectivity, workers::{self, calculate_path, Worker, WorkerArrivedEvent, WorkerPath}};

#[derive(Debug, Clone)]
pub enum TaskAction {
    PickupItems{
        items: HashMap<ItemName, u32>,
    },
    DropItems{
        items: HashMap<ItemName, u32>,
    },
}

#[derive(Component, Clone)]
pub struct WorkerTask {
    pub destination: Option<(i32, i32)>,
    pub task: Option<TaskAction>,
}

impl WorkerTask {
    pub fn has_task(&self) -> bool {
        self.task.is_some()
    }

    pub fn clear(&mut self) {
        self.task = None;
        self.destination = None;
    }
}

#[derive(Event)]
pub struct DesignateTask {
    pub worker: Entity,
    pub task: WorkerTask,
}

const DELIVER_TASK_PRIORITY: u32 = 3;
const PICKUP_TASK_PRIORITY: u32 = 2;
const STORE_TASK_PRIORITY: u32 = 1;

pub fn worker_orchestration_system(
    mut task_events: EventWriter<DesignateTask>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>, Option<&Hub>), With<Building>>,
    workers: Query<(Entity, &Inventory, &WorkerTask), With<Worker>>,
    recipe_registry: Res<RecipeRegistry>,
    workers_en_route: Query<&mut WorkersEnRoute>,
) {
    let mut available_empty_workers: Vec<_> = workers
        .iter()
        .filter(|(_, inventory, task)| !task.has_task() && inventory.items.is_empty())
        .map(|(entity, inventory, _)| (entity, inventory))
        .collect();

    let mut available_workers_with_items: Vec<_> = workers
        .iter()
        .filter(|(_, inventory, task)| !task.has_task() && !inventory.items.is_empty())
        .map(|(entity, inventory, _)| (entity, inventory))
        .collect();

    let deliver_to_requesters_tasks: Vec<_> = buildings
        .iter()
        .filter_map(|(entity, pos, inventory, _, recipe_crafter, _)| {
            if let Some(crafter) = recipe_crafter {
                if let Some(recipe_def) = recipe_registry.get_definition(&crafter.recipe) {
                    let recipe_requirements = recipe_def.inputs.iter().map(|(item_name, quantity)| (item_name.clone(), 3 * quantity)).collect::<HashMap<_, _>>();
                    if !inventory.has_items_for_recipe(&recipe_requirements) {
                        return Some((entity, pos, recipe_requirements));
                    }
                }
            }
            None
        }).collect();

    let pickup_from_senders_tasks: Vec<_> = buildings
        .iter()
        .filter_map(|(entity, pos, inventory, inv_type, recipe_crafter, _)| {
            if !inventory.items.is_empty() && inv_type.0 == InventoryTypes::Sender {
                Some((entity, pos, inventory, inv_type, recipe_crafter))
            } else {
                None
            }
        }).collect();

    deliver_to_requesters(&deliver_to_requesters_tasks, &mut available_workers_with_items, &mut task_events);
    retrieve_from_senders(&pickup_from_senders_tasks, &mut available_empty_workers, &mut task_events, &workers_en_route);
    store_items(&buildings, &mut available_workers_with_items, &mut task_events);
    retrieve_items(&buildings, &mut available_empty_workers, &mut task_events);
}

fn deliver_to_requesters(
    deliver_tasks: &[(Entity, &Position, HashMap<ItemName, u32>)],
    available_workers: &mut Vec<(Entity, &Inventory)>,
    task_events: &mut EventWriter<DesignateTask>,
) {
    for (_, building_pos, needed_items) in deliver_tasks {
        // Find a worker that has at least some of the needed items
        if let Some(worker_idx) = available_workers.iter().position(|(_, inventory)| {
            needed_items.keys().any(|item_name| inventory.get_item_quantity(item_name) > 0)
        }) {
            let (worker_entity, worker_inventory) = available_workers.remove(worker_idx);
            
            // Calculate what items this worker can actually deliver
            let mut items_to_deliver = HashMap::new();
            for (item_name, needed_quantity) in needed_items {
                let worker_has = worker_inventory.get_item_quantity(item_name);
                if worker_has > 0 {
                    items_to_deliver.insert(item_name.clone(), worker_has.min(*needed_quantity));
                }
            }
            
            if !items_to_deliver.is_empty() {
                task_events.send(DesignateTask {
                    worker: worker_entity,
                    task: WorkerTask {
                        destination: Some((building_pos.x, building_pos.y)),
                        task: Some(TaskAction::DropItems { items: items_to_deliver }),
                    },
                });
            }
        }
    }
}

fn retrieve_from_senders(
    pickup_tasks: &[(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>)],
    available_workers: &mut Vec<(Entity, &Inventory)>,
    task_events: &mut EventWriter<DesignateTask>,
    workers_en_route: &Query<&mut WorkersEnRoute>,
) {
    for (building_entity, building_pos, building_inventory, _, _) in pickup_tasks {
        let workers_coming = workers_en_route.get(*building_entity)
            .map(|enroute| enroute.count)
            .unwrap_or(0);
        
        if workers_coming > building_inventory.get_total_quantity() as u32 / 20 { // 20 items per worker
            continue; // Skip this building, workers already en route
        }
        if let Some(worker_idx) = available_workers.iter().position(|(_, _)| true) { // First available worker
            let (worker_entity, _) = available_workers.remove(worker_idx);
            
            // Pick up all available items from the sender
            let items_to_pickup = building_inventory.get_all_items();
            
            if !items_to_pickup.is_empty() && items_to_pickup.keys().any(|item_name| building_inventory.get_item_quantity(item_name) > 10) {
                task_events.send(DesignateTask {
                    worker: worker_entity,
                    task: WorkerTask {
                        destination: Some((building_pos.x, building_pos.y)),
                        task: Some(TaskAction::PickupItems { items: items_to_pickup }),
                    },
                });
            }
        }
    }
}

fn store_items(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>, Option<&Hub>), With<Building>>,
    available_workers: &mut Vec<(Entity, &Inventory)>,
    task_events: &mut EventWriter<DesignateTask>,
) {
    // Find storage buildings
    let storage_buildings: Vec<_> = buildings
        .iter()
        .filter(|(_, _, _, inv_type, _, _)| inv_type.0 == InventoryTypes::Storage)
        .collect();
    
    for (worker_entity, worker_inventory) in available_workers.drain(..) {
        if let Some((_, storage_pos, _, _, _, _)) = storage_buildings.first() {
            // Send worker to first available storage
            let items_to_store = worker_inventory.get_all_items();
            
            if !items_to_store.is_empty() {
                task_events.send(DesignateTask {
                    worker: worker_entity,
                    task: WorkerTask {
                        destination: Some((storage_pos.x, storage_pos.y)),
                        task: Some(TaskAction::DropItems { items: items_to_store }),
                    },
                });
            }
        }
    }
}

fn retrieve_items(
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>, Option<&Hub>), With<Building>>,
    available_workers: &mut Vec<(Entity, &Inventory)>,
    task_events: &mut EventWriter<DesignateTask>,
) {
    // Find storage buildings with items and the hub
    let storage_with_items: Vec<_> = buildings
        .iter()
        .filter(|(_, _, inventory, inv_type, _, hub)| {
            inv_type.0 == InventoryTypes::Storage && inventory.has_any_item() && hub.is_none()
        })
        .collect();
    
    let hub_position = buildings
        .iter()
        .find(|(_, _, _, _, _, hub)| hub.is_some())
        .map(|(_, pos, _, _, _, _)| (pos.x, pos.y));
    
    if let Some(hub_pos) = hub_position {
        for (storage_entity, storage_pos, storage_inventory, _, _, _) in storage_with_items {
            if let Some(worker_idx) = available_workers.iter().position(|(_, _)| true) {
                let (worker_entity, _) = available_workers.remove(worker_idx);
                
                // Pick up items from storage to bring to hub
                let items_to_retrieve = storage_inventory.get_all_items();
                
                if !items_to_retrieve.is_empty() {
                    task_events.send(DesignateTask {
                        worker: worker_entity,
                        task: WorkerTask {
                            destination: Some((storage_pos.x, storage_pos.y)),
                            task: Some(TaskAction::PickupItems { items: items_to_retrieve }),
                        },
                    });
                }
            }
        }
    }
}

pub fn handle_worker_paths(
    mut task_events: EventReader<DesignateTask>,
    mut workers: Query<(&mut WorkerTask, &mut WorkerPath, &Transform), With<Worker>>,
    network: Res<NetworkConnectivity>,
    grid: Res<Grid>,
) {
    for task_event in task_events.read() {
        if let Ok((mut worker_task, mut worker_path, transform)) = workers.get_mut(task_event.worker) {
            // Update the worker's task
            *worker_task = task_event.task.clone();
            
            // Calculate path to destination if we have one
            if let Some(destination) = task_event.task.destination {
                // Get worker's current grid position
                if let Some(current_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                    let current_pos = (current_coords.grid_x, current_coords.grid_y);
                    
                    // Calculate path to destination
                    if let Some(new_path) = calculate_path(current_pos, destination, &network, &grid) {
                        worker_path.waypoints = new_path;
                        worker_path.current_target = worker_path.waypoints.pop_front();
                    } else {
                        // No path found - clear the task
                        println!("No path found for worker to destination {:?}", destination);
                        worker_task.clear();
                    }
                } else {
                    println!("Worker not on valid grid position");
                    worker_task.clear();
                }
            }
        }
    }
}

pub fn handle_worker_task_arrivals(
    mut arrivals: EventReader<WorkerArrivedEvent>,
    mut transfer_events: EventWriter<ItemTransferRequestEvent>,
    mut workers: Query<&mut WorkerTask, With<Worker>>,
    mut buildings: Query<(Entity, &Position, &mut WorkersEnRoute), With<Building>>,
    inventories: Query<&Inventory>,
) {
    for arrival in arrivals.read() {
        // Get the worker's current task
        if let Ok(mut worker_task) = workers.get_mut(arrival.worker) {
            if let Some(task_action) = &worker_task.task {
                // Find the building at the arrival position
                if let Some((building_entity, _, _)) = buildings.iter()
                    .find(|(_, pos, _)| pos.x == arrival.position.0 && pos.y == arrival.position.1)
                {
                    match task_action {
                        TaskAction::PickupItems { items } => {
                            // Request to transfer items from building to worker
                            transfer_events.send(ItemTransferRequestEvent {
                                sender: building_entity,
                                receiver: arrival.worker,
                                items: items.clone(),
                            });
                        }
                        TaskAction::DropItems { items } => {
                            // Request to transfer items from worker to building
                            transfer_events.send(ItemTransferRequestEvent {
                                sender: arrival.worker,
                                receiver: building_entity,
                                items: items.clone(),
                            });
                        }
                    }
                    
                    // Clear the task after requesting the transfer
                    worker_task.clear();
                    if let Ok(mut en_route) = buildings.get_mut(building_entity) {
                        en_route.2.count = en_route.2.count.saturating_sub(1);
                    }
                } else {
                    println!("No building found at arrival position {:?}", arrival.position);
                    // Clear task even if no building found to prevent getting stuck
                    worker_task.clear();
                }
            }
        }
    }
}