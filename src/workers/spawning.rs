use std::collections::{HashSet, VecDeque};
use bevy::prelude::*;
use crate::{
    grid::{Grid, Position}, items::Inventory, structures::{Building, BuildingId, Hub, MultiCellBuilding, MINING_DRILL}, systems::Operational, workers::WorkerPath
};

#[derive(Component)]
pub struct Worker;

#[derive(Component)]
pub struct Speed {
    pub value: f32,
}

#[derive(Component)]
pub struct Harvester {
    pub entity: Entity,
}

#[derive(Bundle)]
pub struct WorkerBundle {
    pub worker: Worker,
    pub speed: Speed,
    pub harvester: Harvester,
    pub path: WorkerPath,
    pub inventory: Inventory,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl WorkerBundle {
    pub fn new(harvester_entity: Entity, spawn_position: Vec2) -> Self {
        WorkerBundle {
            worker: Worker,
            speed: Speed { value: 100.0 }, // pixels per second
            harvester: Harvester { entity: harvester_entity },
            path: WorkerPath {
                waypoints: VecDeque::new(),
                current_target: None,
            },
            inventory: Inventory::new(20),
            sprite: Sprite::from_color(Color::srgb(0.4, 0.2, 0.1), Vec2::new(16.0, 16.0)),
            transform: Transform::from_xyz(spawn_position.x, spawn_position.y, 1.5),
        }
    }
}

pub fn spawn_workers_for_new_harvesters(
    mut commands: Commands,
    harvesters: Query<(Entity, &Position, &Operational, &Building), (With<Building>, Changed<Operational>)>,
    existing_workers: Query<&Harvester, With<Worker>>,
    hub_query: Query<&MultiCellBuilding, With<Hub>>,
    grid: Res<Grid>,
) {
    let hub = hub_query.single();
    let hub_world_pos = grid.grid_to_world_coordinates(hub.center_x, hub.center_y);
    
    // Get set of harvesters that already have workers
    let harvesters_with_workers: HashSet<Entity> = existing_workers
        .iter()
        .map(|h| h.entity)
        .collect();
    
    for (entity, _position, operational, building) in harvesters.iter() {
        // Only spawn worker for harvesters that just became operational and don't have workers yet
        if building.id == MINING_DRILL
            && operational.0 
            && !harvesters_with_workers.contains(&entity) {
            
            commands.spawn(WorkerBundle::new(entity, hub_world_pos));
            println!("Spawned worker for harvester {:?}", entity);
        }
    }
}

pub fn despawn_workers_for_removed_harvesters(
    mut commands: Commands,
    workers: Query<(Entity, &Harvester), With<Worker>>,
    harvesters: Query<&Operational, With<Building>>,
) {
    for (worker_entity, harvester_component) in workers.iter() {
        // Check if harvester still exists
        if let Err(_) = harvesters.get(harvester_component.entity) {
            // Harvester no longer exists
            commands.entity(worker_entity).despawn();
            println!("Despawned worker for removed harvester {:?}", harvester_component.entity);
        }
    }
}