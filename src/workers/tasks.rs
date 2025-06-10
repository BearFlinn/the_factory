use std::collections::HashMap;

use bevy::{prelude::*};

use crate::{
    grid::{Grid, Position},
    materials::{request_transfer_all_items, request_transfer_specific_items, Inventory, InventoryType, InventoryTypes, ItemName, ItemTransferRequestEvent, RecipeRegistry},
    structures::{Building, CrafterLogisticsRequest, RecipeCrafter},
    systems::NetworkConnectivity,
    workers::{calculate_path, manhattan_distance_coords, Worker, WorkerArrivedEvent, WorkerPath, WorkerState, WorkerTaskInfo, WorkerTasks}
};

#[derive(Component)]
pub struct Task;

#[derive(Component)]
#[allow(dead_code)] // TODO: Implement priority
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Component, PartialEq)]
pub enum TaskStatus {
    Pending,
    Queued,
    InProgress,
    Completed,
}

#[derive(Component, Clone, Debug)]
pub enum TaskAction {
    Pickup(Option<HashMap<ItemName, u32>>),
    Dropoff(Option<HashMap<ItemName, u32>>),
}

#[derive(Component)]
pub struct TaskTarget(pub Entity);

#[derive(Component)]
pub struct AssignedWorker(pub Option<Entity>);

#[derive(Bundle)]
pub struct TaskBundle {
    task: Task,
    priority: Priority,
    position: Position,
    task_status: TaskStatus,
    task_target: TaskTarget,
    task_action: TaskAction,
    assigned_worker: AssignedWorker,
}

impl TaskBundle {
    pub fn new(task_target: Entity, position: Position, action: TaskAction, priority: Priority) -> Self {
        Self {
            task: Task,
            priority: priority,
            position: position,
            task_status: TaskStatus::Pending,
            task_target: TaskTarget(task_target),
            task_action: action,
            assigned_worker: AssignedWorker(None),
        }
    }
}

pub fn process_worker_tasks(
    mut workers: Query<(&Transform, &mut WorkerTasks, &mut WorkerPath), With<Worker>>,
    mut tasks: Query<&mut TaskStatus, With<Task>>,
    grid: Res<Grid>,
    network: Res<NetworkConnectivity>,
) {
    for (transform, worker_tasks, mut worker_path) in workers.iter_mut() {
        // Update task statuses
        for (index, task_info) in worker_tasks.0.iter().enumerate() {
            if let Ok(mut task_status) = tasks.get_mut(task_info.task) {
                if index == 0 {
                    // FIX: First task should be InProgress when worker starts working on it
                    if *task_status == TaskStatus::Pending || *task_status == TaskStatus::Queued {
                        *task_status = TaskStatus::InProgress;
                    }
                } else {
                    // Subsequent tasks should be queued
                    if *task_status == TaskStatus::Pending {
                        *task_status = TaskStatus::Queued;
                    }
                }
            }
        }

        // Skip pathfinding if already moving
        if worker_path.current_target.is_some() || !worker_path.waypoints.is_empty() {
            continue;
        }

        // Start pathfinding for next task
        if let Some(current_task) = worker_tasks.0.front() {
            if let Some(worker_coords) = grid.world_to_grid_coordinates(transform.translation.truncate()) {
                let worker_pos = (worker_coords.grid_x, worker_coords.grid_y);
                let target_pos = (current_task.position.x, current_task.position.y);

                if let Some(path) = calculate_path(worker_pos, target_pos, &network, &grid) {
                    worker_path.waypoints = path;
                    worker_path.current_target = worker_path.waypoints.pop_front();
                }
            }
        }
    }
}

pub fn handle_task_arrivals(
    mut arrival_events: EventReader<WorkerArrivedEvent>,
    mut workers: Query<(&mut WorkerTasks, &Inventory, &mut WorkerState), With<Worker>>,
    mut tasks: Query<&mut TaskStatus, With<Task>>,
    inventories: Query<&Inventory>,
    mut transfer_requests: EventWriter<ItemTransferRequestEvent>,
) {
    for event in arrival_events.read() {
        if let Ok((mut worker_tasks, worker_inventory, mut worker_state)) = workers.get_mut(event.worker) {
            if let Some(current_task) = worker_tasks.0.front() {
                let task_pos = (current_task.position.x, current_task.position.y);
                
                if event.position == task_pos {
                    match &current_task.action {
                        TaskAction::Pickup(items) => {
                            if let Some(items) = items {
                                request_transfer_specific_items(
                                    current_task.target,
                                    event.worker,
                                    items.clone(),
                                    &mut transfer_requests,
                                );
                            } else {
                                request_transfer_all_items(
                                    current_task.target,
                                    event.worker,
                                    &mut transfer_requests,
                                    &inventories,
                                );
                            }
                        }
                        TaskAction::Dropoff(items) => {
                            if let Some(items) = items {
                                request_transfer_specific_items(
                                    event.worker,
                                    current_task.target,
                                    items.clone(),
                                    &mut transfer_requests,
                                );
                            } else {
                                let all_items = worker_inventory.get_all_items();
                                if !all_items.is_empty() {
                                    request_transfer_specific_items(
                                        event.worker,
                                        current_task.target,
                                        all_items,
                                        &mut transfer_requests,
                                    );
                                }
                            }
                        }
                    }

                    if let Ok(mut task_status) = tasks.get_mut(current_task.task) {
                        *task_status = TaskStatus::Completed;
                    }
                    worker_tasks.0.pop_front();
                    
                    if worker_tasks.0.is_empty() {
                        *worker_state = WorkerState::Idle;
                    }
                }
            }
        }
    }
}

pub fn assign_available_workers_to_tasks(
    mut tasks: Query<(Entity, &mut AssignedWorker, &Position, &Priority), (With<Task>, With<TaskStatus>)>,
    task_statuses: Query<&TaskStatus>,
    workers: Query<(Entity, &Position, &WorkerState, &Inventory), With<Worker>>,
) {
    // Get unassigned pending tasks, sorted by priority
    let mut unassigned_tasks: Vec<_> = tasks
        .iter_mut()
        .filter(|(entity, assigned_worker, _, _)| {
            assigned_worker.0.is_none() && 
            task_statuses.get(*entity).map(|s| *s == TaskStatus::Pending).unwrap_or(false)
        })
        .collect();
    
    // Sort by priority (Critical first, then High, Medium, Low)
    unassigned_tasks.sort_by_key(|(_, _, _, priority)| {
        match priority {
            Priority::Critical => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
        }
    });
    
    for (_, mut assigned_worker, task_position, _) in unassigned_tasks {
        if let Some((worker_entity, _)) = find_available_worker((task_position.x, task_position.y), &workers) {
            assigned_worker.0 = Some(worker_entity);
        }
    }
}

pub fn find_available_worker(
    position: (i32, i32),
    workers: &Query<(Entity, &Position, &WorkerState, &Inventory), With<Worker>>,
) -> Option<(Entity, Position)> {
    let mut best_worker = None;
    let mut closest_distance = i32::MAX;
    
    for (entity, pos, worker_state, inventory) in workers.iter() {
        // Accept idle workers OR workers with light task loads (you can adjust this logic)
        let is_available = *worker_state == WorkerState::Idle || 
                          (inventory.is_empty() && *worker_state == WorkerState::Working);
        
        if is_available {
            let distance = manhattan_distance_coords(position, (pos.x, pos.y));
            if distance < closest_distance {
                closest_distance = distance;
                best_worker = Some((entity, *pos));
            }
        }
    }
    best_worker
}

pub fn assign_worker_to_tasks(
    tasks: Query<(Entity, &AssignedWorker, &TaskTarget, &Position, &TaskAction), With<Task>>,
    mut workers: Query<(Entity, &mut WorkerTasks, &mut WorkerState), With<Worker>>,
) {
    for (task_entity, assigned_worker, task_target, task_position, task_action) in tasks.iter() {
        if let Some(worker_entity) = assigned_worker.0 {
            if let Ok((_, mut worker_tasks, mut worker_state)) = workers.get_mut(worker_entity) {
                let already_assigned = worker_tasks.0.iter().any(|task_info| task_info.task == task_entity);
                
                if !already_assigned {
                    worker_tasks.0.push_back(WorkerTaskInfo { 
                        task: task_entity, 
                        target: task_target.0, 
                        position: task_position.clone(), 
                        action: task_action.clone() 
                    });
                    *worker_state = WorkerState::Working;
                }
            }
        }
    }
}

pub fn create_logistics_tasks(
    mut commands: Commands,
    mut events: EventReader<CrafterLogisticsRequest>,
    buildings: Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    workers: Query<(Entity, &Position, &mut WorkerTasks, &mut WorkerState, &Inventory), With<Worker>>,
    recipes: Res<RecipeRegistry>,
) {
    for event in events.read() {
        match (&event.needs, &event.has) {
            (Some(_), None) => {
                if let Some((worker_entity, _)) = find_closest_idle_worker((event.position.x, event.position.y), &workers) {
                    if let Some(recipe_crafter) = buildings.get(event.crafter).unwrap().4 {
                        let recipe_inputs = recipes.get_inputs(&recipe_crafter.recipe).unwrap();
                        for (item_name, needed_quantity) in recipe_inputs.iter() {
                            let mut item_requirements = HashMap::new();
                            item_requirements.insert(item_name.clone(), needed_quantity.clone());
                            let closest_buildings = find_closest_buildings_with_recipe_ingredients(
                                (event.position.x, event.position.y),
                                &item_requirements,
                                &buildings,
                            );
                            for (building_entity, building_pos) in closest_buildings {
                                commands
                                    .spawn(TaskBundle::new(
                                        building_entity,
                                        building_pos,
                                        TaskAction::Pickup(Some(item_requirements.clone())),
                                        Priority::Medium,
                                    ))
                                    .insert(AssignedWorker(Some(worker_entity)));
                                
                                commands
                                    .spawn(TaskBundle::new(
                                        event.crafter,
                                        event.position,
                                        TaskAction::Dropoff(Some(item_requirements.clone())),
                                        Priority::Medium,
                                    ))
                                    .insert(AssignedWorker(Some(worker_entity)));
                            }
                        }
                    }
                }
            }
            (None, Some(_)) => {
                if let Some((worker_entity, _)) = find_closest_idle_worker((event.position.x, event.position.y), &workers) {
                    commands
                        .spawn(TaskBundle::new(
                            event.crafter,
                            event.position,
                            TaskAction::Pickup(event.has.clone()),
                            Priority::Medium,
                        ))
                        .insert(AssignedWorker(Some(worker_entity)));
                    
                    if let Ok(worker_inventory) = workers.get(worker_entity).map(|(_, _, _, _, inv)| inv) {
                        if let Some((receiver_entity, receiver_pos)) = find_closest_item_receiver(
                            (event.position.x, event.position.y),
                            worker_inventory,
                            &buildings,
                            &recipes
                        ) {
                            commands
                                .spawn(TaskBundle::new(
                                    receiver_entity,
                                    receiver_pos,
                                    TaskAction::Dropoff(None),
                                    Priority::Medium,
                                ))
                                .insert(AssignedWorker(Some(worker_entity)));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn find_closest_buildings_with_recipe_ingredients(
    position: (i32, i32),
    recipe_inputs: &HashMap<ItemName, u32>,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
) -> Vec<(Entity, Position)> {
    let mut item_requirements = recipe_inputs.clone();
    let mut result = Vec::new();

    loop {
        if item_requirements.is_empty() {
            break;
        }

        let mut closest_building: Option<(Entity, Position)> = None;
        let mut closest_distance = i32::MAX;

        for (entity, pos, inventory, inv_type, _) in buildings.iter() {
            if inv_type.0 != InventoryTypes::Storage && inv_type.0 != InventoryTypes::Sender {
                continue;
            }

            let mut can_fulfill = false;
            for (item_name, needed_quantity) in item_requirements.iter() {
                let available_quantity = inventory.get_item_quantity(item_name); 

                if available_quantity >= *needed_quantity {
                    can_fulfill = true;
                    break;
                }
            }

            if can_fulfill {
                let distance = manhattan_distance_coords(position, (pos.x, pos.y));
                if distance < closest_distance {
                    closest_distance = distance;
                    closest_building = Some((entity, *pos));
                }
            }
        }

        if let Some((entity, pos)) = closest_building {
            result.push((entity, pos));

            if let Ok((_, _, inventory, _, _)) = buildings.get(entity) {
                for (item_name, needed_quantity) in item_requirements.clone().iter() {
                    let available_quantity = inventory.get_item_quantity(item_name); 

                    if available_quantity >= *needed_quantity {
                        item_requirements.remove(item_name);
                    } else {
                        *item_requirements.get_mut(item_name).unwrap() -= available_quantity;
                    }
                    
                }
            }
        } else {
            break;
        }
    }

    result
}


pub fn find_closest_idle_worker(
    position: (i32, i32),
    workers: &Query<(Entity, &Position, &mut WorkerTasks, &mut WorkerState, &Inventory), With<Worker>>,
) -> Option<(Entity, Position)> {
    let mut closest_worker = None;
    let mut closest_distance = i32::MAX;
    for (entity, pos, _, worker_state, inventory) in workers.iter() {
        if worker_state == &WorkerState::Idle && inventory.is_empty() {
            let distance = manhattan_distance_coords(position, (pos.x, pos.y));
            if distance < closest_distance {
                closest_distance = distance;
                closest_worker = Some((entity, *pos));
            }
        }
    }
    closest_worker
}

pub fn find_closest_item_receiver(
    position: (i32, i32),
    inventory: &Inventory,
    buildings: &Query<(Entity, &Position, &Inventory, &InventoryType, Option<&RecipeCrafter>), With<Building>>,
    recipes: &Res<RecipeRegistry>,
) -> Option<(Entity, Position)> {
    let mut closest_building = None;
    let mut closest_distance = i32::MAX;
    for (entity, pos, inv, inv_type, recipe_crafter) in buildings.iter() {
        if let Some(recipe_crafter) = recipe_crafter {
            if inv_type.0 == InventoryTypes::Requester || inv_type.0 == InventoryTypes::Producer {
                for (item_name, _) in recipes.get_inputs(&recipe_crafter.recipe).unwrap().iter() {
                    if inventory.has_item(item_name) {
                        let distance = manhattan_distance_coords((pos.x, pos.y), position);
                        if distance < closest_distance {
                            closest_building = Some((entity, *pos));
                            closest_distance = distance;
                        }
                    }
                }
            }
        }
        if inv_type.0 == InventoryTypes::Storage {
            if inv.has_space_for(&inventory.items) {
                let distance = manhattan_distance_coords((pos.x, pos.y), position);
                if distance < closest_distance {
                    closest_building = Some((entity, *pos));
                    closest_distance = distance;
                }
            }
        }
    }
    closest_building
}

pub fn clear_all_tasks(
    mut commands: Commands,
    query: Query<Entity, With<Task>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        for entity in query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn clear_completed_tasks(
    mut commands: Commands, 
    query: Query<(Entity, &TaskStatus), With<Task>>,
) {
    for (entity, status) in query.iter() {
        if *status == TaskStatus::Completed {
            commands.entity(entity).despawn_recursive();
        }
    }
}