use crate::structures::{BuildingComponentDef, BuildingRegistry};
use crate::ui::{BuildingButton, UISystemSet};
use bevy::prelude::*;

#[derive(Component)]
pub struct Tooltip {
    pub content: String,
}

#[derive(Component)]
pub struct TooltipTimer {
    pub timer: Timer,
    pub target_entity: Entity,
}

#[derive(Component)]
pub struct TooltipTarget;

#[derive(Component)]
pub struct TooltipContainer;

impl TooltipTimer {
    pub fn new(target_entity: Entity, delay_seconds: f32) -> Self {
        Self {
            timer: Timer::from_seconds(delay_seconds, TimerMode::Once),
            target_entity,
        }
    }
}

#[allow(clippy::needless_pass_by_value, clippy::type_complexity)] // Bevy system parameters require by-value
pub fn handle_tooltip_hover_detection(
    mut commands: Commands,
    button_query: Query<
        (Entity, &Interaction, &BuildingButton),
        (Changed<Interaction>, With<TooltipTarget>),
    >,
    timer_query: Query<(Entity, &TooltipTimer)>,
    existing_tooltips: Query<Entity, With<Tooltip>>,
) {
    for (button_entity, interaction, _building_button) in button_query.iter() {
        match interaction {
            Interaction::Hovered => {
                // Check if timer already exists for this button
                let timer_exists = timer_query
                    .iter()
                    .any(|(_, timer)| timer.target_entity == button_entity);

                if !timer_exists {
                    // Start hover timer
                    commands.spawn(TooltipTimer::new(button_entity, 0.8));
                }
            }
            Interaction::None => {
                // Remove any existing timer and tooltip for this button
                for (timer_entity, timer) in timer_query.iter() {
                    if timer.target_entity == button_entity {
                        commands.entity(timer_entity).despawn();
                    }
                }

                // Remove any existing tooltips (we'll only have one at a time)
                for tooltip_entity in existing_tooltips.iter() {
                    commands.entity(tooltip_entity).despawn_recursive();
                }
            }
            Interaction::Pressed => {}
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // Bevy system parameters require by-value
pub fn update_tooltip_timers(
    mut commands: Commands,
    mut timer_query: Query<(Entity, &mut TooltipTimer)>,
    button_query: Query<(&BuildingButton, &GlobalTransform), With<TooltipTarget>>,
    registry: Res<BuildingRegistry>,
    time: Res<Time>,
    existing_tooltips: Query<Entity, With<Tooltip>>,
) {
    for (timer_entity, mut tooltip_timer) in &mut timer_query {
        tooltip_timer.timer.tick(time.delta());

        if tooltip_timer.timer.just_finished() {
            // Remove the timer
            commands.entity(timer_entity).despawn();

            // Remove any existing tooltips first
            for tooltip_entity in existing_tooltips.iter() {
                commands.entity(tooltip_entity).despawn_recursive();
            }

            // Get button info and spawn tooltip
            if let Ok((building_button, button_transform)) =
                button_query.get(tooltip_timer.target_entity)
            {
                if let Some(definition) = registry.get_definition(&building_button.building_name) {
                    let tooltip_content = generate_tooltip_content(definition);
                    spawn_tooltip(
                        &mut commands,
                        tooltip_content,
                        button_transform.translation().truncate(),
                    );
                } else {
                    warn!(
                        "Building definition not found for tooltip: {}",
                        building_button.building_name
                    );
                }
            }
        }
    }
}

fn spawn_tooltip(commands: &mut Commands, content: String, position: Vec2) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x + 150.0), // Offset to the right of the sidebar
                top: Val::Px(position.y - 100.0),
                max_width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
            BorderColor(Color::srgb(0.6, 0.6, 0.6)),
            Tooltip {
                content: content.clone(),
            },
            TooltipContainer,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(content),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));
        });
}

fn generate_tooltip_content(definition: &crate::structures::BuildingDef) -> String {
    use std::fmt::Write;

    let mut content = String::new();

    // Building name and category
    let _ = writeln!(content, "{}", definition.name);
    let _ = writeln!(content, "Category: {:?}\n", definition.category);

    // Full cost breakdown
    content.push_str("Cost:\n");
    if definition.placement.cost.inputs.is_empty() {
        content.push_str("  Free\n");
    } else {
        let mut sorted_inputs: Vec<_> = definition.placement.cost.inputs.iter().collect();
        sorted_inputs.sort_by_key(|(name, _)| name.as_str());

        for (item_name, quantity) in sorted_inputs {
            let _ = writeln!(content, "  {quantity} {item_name}");
        }
    }

    // Building capabilities based on components
    content.push_str("\nCapabilities:\n");
    let mut has_capabilities = false;

    for component in &definition.components {
        match component {
            BuildingComponentDef::PowerConsumer { amount } => {
                let _ = writeln!(content, "  - Consumes {amount} power");
                has_capabilities = true;
            }
            BuildingComponentDef::PowerGenerator { amount } => {
                let _ = writeln!(content, "  - Generates {amount} power");
                has_capabilities = true;
            }
            BuildingComponentDef::ComputeGenerator { amount } => {
                let _ = writeln!(content, "  - Generates {amount} compute");
                has_capabilities = true;
            }
            BuildingComponentDef::ComputeConsumer { amount } => {
                let _ = writeln!(content, "  - Consumes {amount} compute");
                has_capabilities = true;
            }
            BuildingComponentDef::Inventory { capacity } => {
                let _ = writeln!(content, "  - Storage capacity: {capacity}");
                has_capabilities = true;
            }
            BuildingComponentDef::InventoryType { inv_type } => {
                let type_desc = match inv_type {
                    crate::materials::InventoryTypes::Storage => "Storage for items",
                    crate::materials::InventoryTypes::Sender => "Sends items to network",
                    crate::materials::InventoryTypes::Requester => "Requests items from network",
                    crate::materials::InventoryTypes::Carrier => "Carries items",
                    crate::materials::InventoryTypes::Producer => "Produces items",
                };
                let _ = writeln!(content, "  - {type_desc}");
                has_capabilities = true;
            }
            BuildingComponentDef::ViewRange { radius } => {
                let _ = writeln!(content, "  - View range: {radius} tiles");
                has_capabilities = true;
            }
            BuildingComponentDef::NetWorkComponent => {
                content.push_str("  - Network connection point\n");
                has_capabilities = true;
            }
            BuildingComponentDef::RecipeCrafter {
                recipe_name,
                available_recipes: _,
                interval,
            } => {
                let name = recipe_name.as_deref().unwrap_or("Unknown");
                let _ = writeln!(content, "  - Crafts '{name}' every {interval:.1}s");
                has_capabilities = true;
            }
            BuildingComponentDef::Scanner { base_scan_interval } => {
                let _ = writeln!(
                    content,
                    "Reveals new tiles every {base_scan_interval:.1}s, scales with distance"
                );
                has_capabilities = true;
            }
        }
    }

    if !has_capabilities {
        content.push_str("  - Basic structure\n");
    }

    content
}

pub struct TooltipsPlugin;

impl Plugin for TooltipsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ((handle_tooltip_hover_detection, update_tooltip_timers)
                .in_set(UISystemSet::EntityManagement),),
        );
    }
}
