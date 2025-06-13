use bevy::prelude::*;
use crate::ui::interaction_handler::InteractiveUI;
use crate::structures::{BuildingRegistry, BuildingComponentDef};
use crate::ui::BuildingButton;
use std::collections::HashMap;

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

pub fn handle_tooltip_hover_detection(
    mut commands: Commands,
    button_query: Query<(Entity, &Interaction, &BuildingButton), (Changed<Interaction>, With<TooltipTarget>)>,
    timer_query: Query<(Entity, &mut TooltipTimer)>,
    existing_tooltips: Query<Entity, With<Tooltip>>,
) {
    for (button_entity, interaction, _building_button) in button_query.iter() {
        match interaction {
            Interaction::Hovered => {
                // Check if timer already exists for this button
                let timer_exists = timer_query.iter().any(|(_, timer)| timer.target_entity == button_entity);
                
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
            _ => {}
        }
    }
}

pub fn update_tooltip_timers(
    mut commands: Commands,
    mut timer_query: Query<(Entity, &mut TooltipTimer)>,
    button_query: Query<(&BuildingButton, &GlobalTransform), With<TooltipTarget>>,
    registry: Res<BuildingRegistry>,
    time: Res<Time>,
    existing_tooltips: Query<Entity, With<Tooltip>>,
) {
    for (timer_entity, mut tooltip_timer) in timer_query.iter_mut() {
        tooltip_timer.timer.tick(time.delta());
        
        if tooltip_timer.timer.just_finished() {
            // Remove the timer
            commands.entity(timer_entity).despawn();
            
            // Remove any existing tooltips first
            for tooltip_entity in existing_tooltips.iter() {
                commands.entity(tooltip_entity).despawn_recursive();
            }
            
            // Get button info and spawn tooltip
            if let Ok((building_button, button_transform)) = button_query.get(tooltip_timer.target_entity) {
                if let Some(definition) = registry.get_definition(&building_button.building_name) {
                    let tooltip_content = generate_tooltip_content(definition);
                    spawn_tooltip(&mut commands, tooltip_content, button_transform.translation().truncate());
                }
            }
        }
    }
}

fn spawn_tooltip(commands: &mut Commands, content: String, position: Vec2) {
    commands.spawn((
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
        Tooltip { content: content.clone() },
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
    let mut content = String::new();
    
    // Building name and category
    content.push_str(&format!("{}\n", definition.name));
    content.push_str(&format!("Category: {:?}\n\n", definition.category));
    
    // Full cost breakdown
    content.push_str("Cost:\n");
    if definition.placement.cost.inputs.is_empty() {
        content.push_str("  Free\n");
    } else {
        let mut sorted_inputs: Vec<_> = definition.placement.cost.inputs.iter().collect();
        sorted_inputs.sort_by_key(|(name, _)| name.as_str());
        
        for (item_name, quantity) in sorted_inputs {
            content.push_str(&format!("  {} {}\n", quantity, item_name));
        }
    }
    
    // Building capabilities based on components
    content.push_str("\nCapabilities:\n");
    let mut has_capabilities = false;
    
    for component in &definition.components {
        match component {
            BuildingComponentDef::PowerConsumer { amount } => {
                content.push_str(&format!("  - Consumes {} power\n", amount));
                has_capabilities = true;
            }
            BuildingComponentDef::PowerGenerator { amount } => {
                content.push_str(&format!("  - Generates {} power\n", amount));
                has_capabilities = true;
            }
            BuildingComponentDef::ComputeGenerator { amount } => {
                content.push_str(&format!("  - Generates {} compute\n", amount));
                has_capabilities = true;
            }
            BuildingComponentDef::ComputeConsumer { amount } => {
                content.push_str(&format!("  - Consumes {} compute\n", amount));
                has_capabilities = true;
            }
            BuildingComponentDef::Inventory { capacity } => {
                content.push_str(&format!("  - Storage capacity: {}\n", capacity));
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
                content.push_str(&format!("  - {}\n", type_desc));
                has_capabilities = true;
            }
            BuildingComponentDef::ViewRange { radius } => {
                content.push_str(&format!("  - View range: {} tiles\n", radius));
                has_capabilities = true;
            }
            BuildingComponentDef::NetWorkComponent => {
                content.push_str("  - Network connection point\n");
                has_capabilities = true;
            }
            BuildingComponentDef::RecipeCrafter { recipe_name, interval } => {
                content.push_str(&format!("  - Crafts '{}' every {:.1}s\n", recipe_name, interval));
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
        app.add_systems(Update, (
            handle_tooltip_hover_detection,
            update_tooltip_timers,
        ).chain());
    }
}