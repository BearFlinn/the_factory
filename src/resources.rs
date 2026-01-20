use crate::{
    constants::{
        gridlayers::RESOURCE_LAYER,
        items::{COAL, COPPER_ORE, IRON_ORE},
    },
    grid::{CellChildren, Grid, Layer, NewCellEvent, Position},
    materials::RecipeName,
};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

#[derive(Component)]
pub struct ResourceNode;

#[derive(Component)]
pub struct ResourceNodeRecipe {
    pub recipe_name: RecipeName,
}

#[derive(Bundle)]
pub struct ResourceNodeBundle {
    node: ResourceNode,
    produces: ResourceNodeRecipe,
    position: Position,
    layer: Layer,
}

impl ResourceNodeBundle {
    pub fn new(x: i32, y: i32, produces: ResourceNodeRecipe) -> Self {
        ResourceNodeBundle {
            node: ResourceNode,
            produces,
            position: Position { x, y },
            layer: Layer(RESOURCE_LAYER),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    Iron,
    Copper,
    Coal,
}

impl OreType {
    pub fn recipe_name(self) -> RecipeName {
        match self {
            OreType::Iron => IRON_ORE.to_string(),
            OreType::Copper => COPPER_ORE.to_string(),
            OreType::Coal => COAL.to_string(),
        }
    }

    pub fn color(self) -> Color {
        match self {
            OreType::Iron => Color::srgb(0.2, 0.3, 0.5),
            OreType::Copper => Color::srgb(0.8, 0.5, 0.2),
            OreType::Coal => Color::srgb(0.2, 0.2, 0.2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OreNoiseConfig {
    pub presence_scale: f64,
    pub presence_threshold: f64,
    pub type_bias_scale: f64,
}

impl Default for OreNoiseConfig {
    fn default() -> Self {
        Self {
            presence_scale: 0.05,
            presence_threshold: 0.3,
            type_bias_scale: 0.02,
        }
    }
}

#[derive(Resource)]
pub struct OreNoise {
    config: OreNoiseConfig,
    presence_noise: Perlin,
    iron_bias_noise: Perlin,
    copper_bias_noise: Perlin,
    coal_bias_noise: Perlin,
}

impl OreNoise {
    pub fn new(seed: u32, config: OreNoiseConfig) -> Self {
        Self {
            config,
            presence_noise: Perlin::new(seed),
            iron_bias_noise: Perlin::new(seed.wrapping_add(1000)),
            copper_bias_noise: Perlin::new(seed.wrapping_add(2000)),
            coal_bias_noise: Perlin::new(seed.wrapping_add(3000)),
        }
    }

    #[must_use]
    pub fn should_spawn_ore(&self, x: i32, y: i32) -> bool {
        let nx = f64::from(x) * self.config.presence_scale;
        let ny = f64::from(y) * self.config.presence_scale;
        let noise_value = self.presence_noise.get([nx, ny]);
        noise_value > self.config.presence_threshold
    }

    #[must_use]
    pub fn select_ore_type(&self, x: i32, y: i32) -> OreType {
        let nx = f64::from(x) * self.config.type_bias_scale;
        let ny = f64::from(y) * self.config.type_bias_scale;

        let iron_bias = self.iron_bias_noise.get([nx, ny]);
        let copper_bias = self.copper_bias_noise.get([nx, ny]);
        let coal_bias = self.coal_bias_noise.get([nx, ny]);

        if iron_bias >= copper_bias && iron_bias >= coal_bias {
            OreType::Iron
        } else if copper_bias >= coal_bias {
            OreType::Copper
        } else {
            OreType::Coal
        }
    }
}

impl Default for OreNoise {
    fn default() -> Self {
        Self::new(42, OreNoiseConfig::default())
    }
}

pub fn spawn_resource_node(
    mut commands: Commands,
    grid: Res<Grid>,
    ore_noise: Res<OreNoise>,
    mut cell_event: EventReader<NewCellEvent>,
    mut grid_cells: Query<(Entity, &Position, &mut CellChildren)>,
) {
    for event in cell_event.read() {
        if !ore_noise.should_spawn_ore(event.x, event.y) {
            continue;
        }

        let world_pos = grid.grid_to_world_coordinates(event.x, event.y);

        let Some((_, _, mut cell_children)) = grid_cells
            .iter_mut()
            .find(|(_, pos, _)| pos.x == event.x && pos.y == event.y)
        else {
            eprintln!("could not find cell at ({}, {})", event.x, event.y);
            continue;
        };

        let ore_type = ore_noise.select_ore_type(event.x, event.y);

        let resource_node = commands
            .spawn(ResourceNodeBundle::new(
                event.x,
                event.y,
                ResourceNodeRecipe {
                    recipe_name: ore_type.recipe_name(),
                },
            ))
            .insert(Sprite::from_color(ore_type.color(), Vec2::new(48.0, 48.0)))
            .insert(Transform::from_xyz(world_pos.x, world_pos.y, 0.2))
            .id();

        cell_children.0.push(resource_node);
    }
}

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OreNoise>()
            .add_systems(Update, spawn_resource_node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    mod ore_noise_tests {
        use super::*;

        #[test]
        fn same_seed_produces_same_results() {
            let noise1 = OreNoise::new(123, OreNoiseConfig::default());
            let noise2 = OreNoise::new(123, OreNoiseConfig::default());

            for x in -50..50 {
                for y in -50..50 {
                    assert_eq!(
                        noise1.should_spawn_ore(x, y),
                        noise2.should_spawn_ore(x, y),
                        "Presence mismatch at ({x}, {y})"
                    );
                    assert_eq!(
                        noise1.select_ore_type(x, y),
                        noise2.select_ore_type(x, y),
                        "Ore type mismatch at ({x}, {y})"
                    );
                }
            }
        }

        #[test]
        fn different_seeds_produce_different_results() {
            let noise1 = OreNoise::new(123, OreNoiseConfig::default());
            let noise2 = OreNoise::new(456, OreNoiseConfig::default());

            let mut presence_differences = 0;
            let mut type_differences = 0;

            for x in -50..50 {
                for y in -50..50 {
                    if noise1.should_spawn_ore(x, y) != noise2.should_spawn_ore(x, y) {
                        presence_differences += 1;
                    }
                    if noise1.select_ore_type(x, y) != noise2.select_ore_type(x, y) {
                        type_differences += 1;
                    }
                }
            }

            assert!(
                presence_differences > 100,
                "Expected significant presence differences, got {presence_differences}"
            );
            assert!(
                type_differences > 100,
                "Expected significant type differences, got {type_differences}"
            );
        }

        #[test]
        fn ore_types_are_clustered() {
            let noise = OreNoise::new(42, OreNoiseConfig::default());
            let mut adjacent_same_type = 0;
            let mut total_adjacent_pairs = 0;

            for x in -50..50 {
                for y in -50..50 {
                    let current_type = noise.select_ore_type(x, y);

                    if noise.select_ore_type(x + 1, y) == current_type {
                        adjacent_same_type += 1;
                    }
                    if noise.select_ore_type(x, y + 1) == current_type {
                        adjacent_same_type += 1;
                    }
                    total_adjacent_pairs += 2;
                }
            }

            let clustering_ratio = f64::from(adjacent_same_type) / f64::from(total_adjacent_pairs);
            assert!(
                clustering_ratio > 0.5,
                "Expected clustering ratio > 0.5 (random would be ~0.33), got {clustering_ratio}"
            );
        }

        #[test]
        fn all_ore_types_spawn() {
            let noise = OreNoise::new(42, OreNoiseConfig::default());
            let mut iron_count = 0;
            let mut copper_count = 0;
            let mut coal_count = 0;

            for x in -100..100 {
                for y in -100..100 {
                    match noise.select_ore_type(x, y) {
                        OreType::Iron => iron_count += 1,
                        OreType::Copper => copper_count += 1,
                        OreType::Coal => coal_count += 1,
                    }
                }
            }

            let total = iron_count + copper_count + coal_count;
            assert!(
                iron_count > total / 10,
                "Expected significant iron spawns, got {iron_count}/{total}"
            );
            assert!(
                copper_count > total / 10,
                "Expected significant copper spawns, got {copper_count}/{total}"
            );
            assert!(
                coal_count > total / 10,
                "Expected significant coal spawns, got {coal_count}/{total}"
            );
        }

        #[test]
        fn presence_creates_clusters() {
            let noise = OreNoise::new(42, OreNoiseConfig::default());
            let mut ore_cells = 0;
            let mut adjacent_ore_pairs = 0;
            let mut total_ore_adjacent_checks = 0;

            for x in -50..50 {
                for y in -50..50 {
                    if noise.should_spawn_ore(x, y) {
                        ore_cells += 1;

                        if noise.should_spawn_ore(x + 1, y) {
                            adjacent_ore_pairs += 1;
                        }
                        total_ore_adjacent_checks += 1;

                        if noise.should_spawn_ore(x, y + 1) {
                            adjacent_ore_pairs += 1;
                        }
                        total_ore_adjacent_checks += 1;
                    }
                }
            }

            assert!(
                ore_cells > 100,
                "Expected ore to spawn, got {ore_cells} cells"
            );

            if total_ore_adjacent_checks > 0 {
                let adjacency_ratio =
                    f64::from(adjacent_ore_pairs) / f64::from(total_ore_adjacent_checks);
                assert!(
                    adjacency_ratio > 0.3,
                    "Expected ore adjacency > 0.3 (indicating clustering), got {adjacency_ratio}"
                );
            }
        }
    }
}
