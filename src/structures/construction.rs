use std::collections::HashMap;

pub use crate::{
    constants::gridlayers::BUILDING_LAYER,
    grid::{CellChildren, Grid, Layer, Position},
    materials::items::{InputPort, InventoryAccess, StoragePort},
    structures::building_config::*,
    systems::Operational,
};
use crate::{
    constants::structures::MINING_DRILL,
    grid::ExpandGridEvent,
    materials::{ItemName, RecipeDef, RecipeName},
    resources::{ResourceNode, ResourceNodeRecipe},
    systems::NetworkChangedEvent,
    workers::{Priority, Task, TaskSequence, TaskTarget},
};

#[derive(Component)]
pub struct Building;

#[derive(Component)]
pub struct ViewRange {
    pub radius: i32,
}

#[derive(Component, Debug)]
pub struct RecipeCrafter {
    pub timer: Timer,
    pub current_recipe: Option<RecipeName>,
    pub available_recipes: Vec<RecipeName>,
}

#[derive(Component, Debug, Default, Clone)]
pub struct RecipeCommitment {
    pub committed_recipe: Option<RecipeName>,
    pub pending_recipe: Option<RecipeName>,
    pub in_transit_items: HashMap<ItemName, u32>,
}

#[derive(Component)]
pub struct NeedsRecipeCommitmentEvaluation;

impl RecipeCommitment {
    pub fn new_committed(recipe: Option<RecipeName>) -> Self {
        Self {
            committed_recipe: recipe,
            pending_recipe: None,
            in_transit_items: HashMap::new(),
        }
    }

    pub fn has_pending_items(&self) -> bool {
        !self.in_transit_items.is_empty()
    }

    pub fn can_commit_new_recipe(&self) -> bool {
        self.in_transit_items.is_empty()
    }

    pub fn add_in_transit(&mut self, items: &HashMap<ItemName, u32>) {
        for (item_name, &amount) in items {
            *self.in_transit_items.entry(item_name.clone()).or_insert(0) += amount;
        }
    }

    pub fn remove_in_transit(&mut self, items: &HashMap<ItemName, u32>) {
        for (item_name, &amount) in items {
            if let Some(current) = self.in_transit_items.get_mut(item_name) {
                *current = current.saturating_sub(amount);
                if *current == 0 {
                    self.in_transit_items.remove(item_name);
                }
            }
        }
    }
}

impl RecipeCrafter {
    pub fn is_single_recipe(&self) -> bool {
        self.available_recipes.is_empty()
    }

    pub fn is_multi_recipe(&self) -> bool {
        !self.available_recipes.is_empty()
    }

    pub fn get_active_recipe(&self) -> Option<&RecipeName> {
        self.current_recipe.as_ref()
    }

    pub fn set_recipe(&mut self, recipe_name: RecipeName) -> Result<(), String> {
        if self.is_single_recipe() || self.available_recipes.contains(&recipe_name) {
            self.current_recipe = Some(recipe_name);
            Ok(())
        } else {
            Err(format!(
                "Recipe '{recipe_name}' not available for this crafter"
            ))
        }
    }
}

#[derive(Component)]
pub struct PowerGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct PowerConsumer {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeGenerator {
    pub amount: i32,
}

#[derive(Component)]
pub struct ComputeConsumer {
    pub amount: i32,
}

#[derive(Component, Clone)]
pub struct BuildingCost {
    pub cost: RecipeDef,
}

#[derive(Component)]
pub struct MultiCellBuilding {
    pub width: i32,
    pub height: i32,
    pub center_x: i32,
    pub center_y: i32,
}

#[derive(Component, PartialEq)]
pub struct NetWorkComponent;

#[derive(Component)]
pub struct PendingDrillRecipeAssignment {
    pub position: Position,
}

#[derive(Component)]
pub struct ConstructionSite {
    pub building_name: String,
}

#[derive(Bundle)]
pub struct ConstructionSiteBundle {
    pub construction_site: ConstructionSite,
    pub building_cost: BuildingCost,
    input_port: InputPort,
    pub position: Position,
    pub layer: Layer,
    pub sprite: Sprite,
    pub transform: Transform,
}

impl ConstructionSiteBundle {
    pub fn new(
        building_name: String,
        building_cost: BuildingCost,
        position: Position,
        world_pos: Vec2,
        appearance: &AppearanceDef,
    ) -> Self {
        Self {
            construction_site: ConstructionSite { building_name },
            building_cost,
            input_port: InputPort::new(1000),
            position,
            layer: Layer(BUILDING_LAYER),
            sprite: Sprite::from_color(
                Color::srgba(
                    appearance.color.0,
                    appearance.color.1,
                    appearance.color.2,
                    0.7,
                ),
                Vec2::new(appearance.size.0, appearance.size.1),
            ),
            transform: Transform::from_xyz(world_pos.x, world_pos.y, 0.8),
        }
    }
}

#[derive(Event)]
pub struct ConstructionMaterialRequest {
    pub site: Entity,
    pub position: Position,
    pub needed_materials: HashMap<ItemName, u32>,
    pub priority: Priority,
}

impl BuildingRegistry {
    pub fn load_from_assets() -> Result<Self, Box<dyn std::error::Error>> {
        let ron_content = include_str!("../assets/buildings.ron");
        Self::from_ron(ron_content)
    }
}

// TODO: Improve Multi-cell building implementation

pub fn occupy_area(
    grid_cells: &mut Query<(Entity, &Position, &mut CellChildren)>,
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
    building_entity: Entity,
) {
    let half_width = width / 2;
    let half_height = height / 2;

    for dy in -half_height..=half_height {
        for dx in -half_width..=half_width {
            let check_x = center_x + dx;
            let check_y = center_y + dy;

            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == check_x && pos.y == check_y)
            {
                cell_children.0.push(building_entity);
            }
        }
    }
}

#[derive(Component)]
pub struct Hub;

pub fn place_hub(
    mut commands: Commands,
    grid: Res<Grid>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    let center_x = 0;
    let center_y = 0;

    let world_pos = grid.grid_to_world_coordinates(center_x, center_y);

    let mut storage_port = StoragePort::new(10000);
    storage_port.add_item("Iron Ore", 400);
    storage_port.add_item("Copper Ore", 400);

    let building_entity = commands
        .spawn((
            Building,
            Hub,
            Position {
                x: center_x,
                y: center_y,
            },
            MultiCellBuilding {
                width: 3,
                height: 3,
                center_x,
                center_y,
            },
            PowerGenerator { amount: 100 },
            ComputeGenerator { amount: 60 },
            storage_port,
            Operational(None),
            Layer(BUILDING_LAYER),
        ))
        .insert(Sprite::from_color(
            Color::srgb(0.3, 0.3, 0.7),
            Vec2::new(120.0, 120.0),
        ))
        .insert(Transform::from_xyz(world_pos.x, world_pos.y, 1.0))
        .id();

    occupy_area(&mut grid_cells, center_x, center_y, 3, 3, building_entity);
}

pub fn monitor_construction_completion(
    mut commands: Commands,
    construction_sites: Query<
        (
            Entity,
            &ConstructionSite,
            &InputPort,
            &BuildingCost,
            &Position,
            &Transform,
        ),
        (Changed<InputPort>, With<ConstructionSite>),
    >,
    registry: Res<BuildingRegistry>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
    mut network_events: EventWriter<NetworkChangedEvent>,
) {
    for (site_entity, construction_site, input_port, building_cost, position, transform) in
        &construction_sites
    {
        if input_port.has_items_for_recipe(&building_cost.cost.inputs) {
            commands.entity(site_entity).despawn();

            if let Some((_, _, mut cell_children)) = grid_cells
                .iter_mut()
                .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y)
            {
                cell_children.0.retain(|&entity| entity != site_entity);
            }

            if let Some(building_entity) = registry.spawn_building(
                &mut commands,
                &construction_site.building_name,
                position.x,
                position.y,
                transform.translation.truncate(),
            ) {
                if construction_site.building_name == MINING_DRILL {
                    commands
                        .entity(building_entity)
                        .insert(PendingDrillRecipeAssignment {
                            position: *position,
                        });
                }
                if let Some((_, _, mut cell_children)) = grid_cells
                    .iter_mut()
                    .find(|(_, pos, _)| pos.x == position.x && pos.y == position.y)
                {
                    cell_children.0.push(building_entity);
                }

                network_events.send(NetworkChangedEvent);
                println!(
                    "Construction completed: {} at ({}, {})",
                    construction_site.building_name, position.x, position.y
                );
            }
        }
    }
}

#[derive(Component)]
pub struct ConstructionMonitor {
    pub last_inventory_snapshot: HashMap<ItemName, u32>,
    pub last_progress_time: f32,
    pub retry_count: u32,
    pub next_retry_time: f32,
}

impl ConstructionMonitor {
    pub fn new(current_inventory: &HashMap<ItemName, u32>, current_time: f32) -> Self {
        Self {
            last_inventory_snapshot: current_inventory.clone(),
            last_progress_time: current_time,
            retry_count: 0,
            next_retry_time: current_time + 10.0, // Initial 10 second check
        }
    }

    pub fn should_retry(&self, current_time: f32) -> bool {
        current_time >= self.next_retry_time
    }

    pub fn schedule_next_retry(&mut self, current_time: f32) {
        self.retry_count += 1;
        self.next_retry_time = current_time + 10.0;

        println!("Construction retry #{} scheduled in 10s", self.retry_count);
    }

    pub fn reset_progress(&mut self, new_inventory: &HashMap<ItemName, u32>, current_time: f32) {
        self.last_inventory_snapshot.clone_from(new_inventory);
        self.last_progress_time = current_time;
        self.retry_count = 0;
        self.next_retry_time = current_time + 10.0;
    }

    pub fn has_made_progress(&self, current_inventory: &HashMap<ItemName, u32>) -> bool {
        for (item_name, &current_amount) in current_inventory {
            let previous_amount = self
                .last_inventory_snapshot
                .get(item_name)
                .copied()
                .unwrap_or(0);
            if current_amount > previous_amount {
                return true;
            }
        }
        false
    }
}

pub fn monitor_construction_progress(
    mut commands: Commands,
    mut construction_sites: Query<
        (
            Entity,
            &ConstructionSite,
            &InputPort,
            &BuildingCost,
            &Position,
            Option<&mut ConstructionMonitor>,
        ),
        With<ConstructionSite>,
    >,
    active_sequences: Query<(&TaskSequence, Entity)>,
    active_tasks: Query<&TaskTarget, With<Task>>,
    mut construction_requests: EventWriter<ConstructionMaterialRequest>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    for (site_entity, construction_site, input_port, building_cost, position, monitor) in
        &mut construction_sites
    {
        let required_materials = &building_cost.cost.inputs;
        let current_materials = input_port.get_all_items();

        if input_port.has_items_for_recipe(required_materials) {
            continue;
        }

        let Some(mut monitor) = monitor else {
            commands
                .entity(site_entity)
                .insert(ConstructionMonitor::new(&current_materials, current_time));
            continue;
        };

        if monitor.has_made_progress(&current_materials) {
            monitor.reset_progress(&current_materials, current_time);
            continue;
        }

        if !monitor.should_retry(current_time) {
            continue;
        }

        let mut still_needed = HashMap::new();
        for (item_name, &required_amount) in required_materials {
            let current_amount = current_materials.get(item_name).copied().unwrap_or(0);
            if current_amount < required_amount {
                still_needed.insert(item_name.clone(), required_amount - current_amount);
            }
        }

        if !still_needed.is_empty() {
            let has_active_tasks = active_sequences.iter().any(|(sequence, _)| {
                if sequence.is_complete() {
                    return false;
                }
                sequence.tasks.iter().any(|&task_entity| {
                    active_tasks
                        .get(task_entity)
                        .is_ok_and(|target| target.0 == site_entity)
                })
            });

            if !has_active_tasks {
                println!(
                    "Construction stalled at ({}, {}): {} - requesting {} materials (retry #{})",
                    position.x,
                    position.y,
                    construction_site.building_name,
                    still_needed.len(),
                    monitor.retry_count + 1
                );

                // Request new supply plan
                construction_requests.send(ConstructionMaterialRequest {
                    site: site_entity,
                    position: *position,
                    needed_materials: still_needed,
                    priority: Priority::High, // Higher priority for retries
                });

                monitor.schedule_next_retry(current_time);
            }
        }
    }
}

pub fn assign_drill_recipes(
    mut commands: Commands,
    mut drills: Query<(Entity, &mut RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
    resource_nodes: Query<(&ResourceNodeRecipe, &Position), With<ResourceNode>>,
) {
    for (drill_entity, mut recipe_crafter, pending) in &mut drills {
        if let Some((resource_recipe, _)) = resource_nodes
            .iter()
            .find(|(_, pos)| pos.x == pending.position.x && pos.y == pending.position.y)
        {
            if let Err(error) = recipe_crafter.set_recipe(resource_recipe.recipe_name.clone()) {
                println!(
                    "Failed to assign recipe to drill at ({}, {}): {}",
                    pending.position.x, pending.position.y, error
                );
            } else {
                commands
                    .entity(drill_entity)
                    .remove::<PendingDrillRecipeAssignment>();
                println!(
                    "Assigned recipe '{}' to drill at ({}, {})",
                    resource_recipe.recipe_name, pending.position.x, pending.position.y
                );
            }
        }
    }
}

pub fn drill_awaiting_assignment(
    drills: Query<(&RecipeCrafter, &PendingDrillRecipeAssignment), With<Building>>,
) -> bool {
    !drills.is_empty()
}

pub fn handle_building_view_range_expansion(
    buildings_with_view_range: Query<(&ViewRange, &Position), Added<Building>>,
    mut expand_events: EventWriter<ExpandGridEvent>,
) {
    for (view_range, position) in &buildings_with_view_range {
        if view_range.radius > 0 {
            expand_events.send(ExpandGridEvent {
                center_x: position.x,
                center_y: position.y,
                radius: view_range.radius,
            });

            println!(
                "Expanding grid for building at ({}, {}) with radius {}",
                position.x, position.y, view_range.radius
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn recipe_commitment_new_committed_with_recipe() {
        let commitment = RecipeCommitment::new_committed(Some("iron_ingot".to_string()));

        assert_eq!(commitment.committed_recipe, Some("iron_ingot".to_string()));
        assert_eq!(commitment.pending_recipe, None);
        assert!(commitment.in_transit_items.is_empty());
    }

    #[test]
    fn recipe_commitment_new_committed_without_recipe() {
        let commitment = RecipeCommitment::new_committed(None);

        assert_eq!(commitment.committed_recipe, None);
        assert_eq!(commitment.pending_recipe, None);
        assert!(commitment.in_transit_items.is_empty());
    }

    #[test]
    fn recipe_commitment_has_pending_items_true_when_items_present() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 10);

        assert!(commitment.has_pending_items());
    }

    #[test]
    fn recipe_commitment_has_pending_items_false_when_empty() {
        let commitment = RecipeCommitment::default();

        assert!(!commitment.has_pending_items());
    }

    #[test]
    fn recipe_commitment_can_commit_new_recipe_true_when_empty() {
        let commitment = RecipeCommitment::default();

        assert!(commitment.can_commit_new_recipe());
    }

    #[test]
    fn recipe_commitment_can_commit_new_recipe_false_when_items_in_transit() {
        let mut commitment = RecipeCommitment::default();
        commitment.in_transit_items.insert("coal".to_string(), 5);

        assert!(!commitment.can_commit_new_recipe());
    }

    #[test]
    fn recipe_commitment_add_in_transit_accumulates_items() {
        let mut commitment = RecipeCommitment::default();
        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 10);
        items.insert("coal".to_string(), 5);

        commitment.add_in_transit(&items);

        assert_eq!(commitment.in_transit_items.get("iron_ore"), Some(&10));
        assert_eq!(commitment.in_transit_items.get("coal"), Some(&5));
    }

    #[test]
    fn recipe_commitment_add_in_transit_accumulates_to_existing() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 5);

        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 10);

        commitment.add_in_transit(&items);

        assert_eq!(commitment.in_transit_items.get("iron_ore"), Some(&15));
    }

    #[test]
    fn recipe_commitment_remove_in_transit_decrements_items() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 20);
        commitment.in_transit_items.insert("coal".to_string(), 10);

        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 5);
        items.insert("coal".to_string(), 3);

        commitment.remove_in_transit(&items);

        assert_eq!(commitment.in_transit_items.get("iron_ore"), Some(&15));
        assert_eq!(commitment.in_transit_items.get("coal"), Some(&7));
    }

    #[test]
    fn recipe_commitment_remove_in_transit_removes_when_zero() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 10);

        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 10);

        commitment.remove_in_transit(&items);

        assert!(!commitment.in_transit_items.contains_key("iron_ore"));
    }

    #[test]
    fn recipe_commitment_remove_in_transit_saturating_sub_prevents_underflow() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 5);

        let mut items = HashMap::new();
        items.insert("iron_ore".to_string(), 20);

        commitment.remove_in_transit(&items);

        assert!(!commitment.in_transit_items.contains_key("iron_ore"));
    }

    #[test]
    fn recipe_commitment_remove_in_transit_ignores_unknown_items() {
        let mut commitment = RecipeCommitment::default();
        commitment
            .in_transit_items
            .insert("iron_ore".to_string(), 10);

        let mut items = HashMap::new();
        items.insert("copper_ore".to_string(), 5);

        commitment.remove_in_transit(&items);

        assert_eq!(commitment.in_transit_items.get("iron_ore"), Some(&10));
        assert!(!commitment.in_transit_items.contains_key("copper_ore"));
    }
}
