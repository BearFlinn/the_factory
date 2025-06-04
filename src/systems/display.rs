use bevy::prelude::*;
use crate::{structures::Building, workers::Worker, items::Inventory, systems::Operational};

#[derive(Component)]
pub struct InventoryDisplay;

pub fn update_inventory_display(
    mut commands: Commands,
    buildings_and_workers: Query<(Entity, &Inventory), Or<(With<Building>, With<Worker>)>>,
    mut inventory_displays: Query<&mut Text2d, With<InventoryDisplay>>,
    children: Query<&Children>,
    changed_inventories: Query<Entity, (Or<(With<Worker>, With<Building>)>, Changed<Inventory>)>,
) {
    for (building_entity, inventory) in buildings_and_workers.iter() {
        // Check if this building's inventory changed, or if we need to create initial display
        let should_update = changed_inventories.contains(building_entity);
        
        let existing_display = children.get(building_entity)
            .ok()
            .and_then(|children| {
                children.iter().find_map(|&child| {
                    if inventory_displays.contains(child) {
                        Some(child)
                    } else {
                        None
                    }
                })
            });

        match existing_display {
            Some(display_entity) => {
                // Update existing display if inventory changed
                if should_update {
                    if let Ok(mut text) = inventory_displays.get_mut(display_entity) {
                        text.0 = format!("{}", inventory.get_item_quantity(0));
                    }
                }
            }
            None => {
                // Create new display
                let display = commands.spawn((
                    InventoryDisplay,
                    Text2d::new(format!("{}", inventory.get_item_quantity(0))),
                    TextFont {
                        font_size: 16.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    Transform::from_xyz(0.0, 0.0, 1.1), // Position above building
                )).id();

                commands.entity(building_entity).add_child(display);
            }
        }
    }
}

#[derive(Component)]
pub struct NonOperationalIndicator;

pub fn update_operational_indicators(
    mut commands: Commands,
    mut buildings: Query<(Entity, &Operational), (With<Building>, Changed<Operational>)>,
    indicators: Query<Entity, With<NonOperationalIndicator>>,
    children: Query<&Children>,
) {
    for (building_entity, operational) in buildings.iter_mut() {
        let existing_indicator = children.get(building_entity)
            .ok()
            .and_then(|children| {
                children.iter().find(|&&child| indicators.contains(child))
            });

        match (operational.0, existing_indicator) {
            (false, None) => {
                let indicator = commands.spawn((
                    NonOperationalIndicator,
                    Text2d("!".to_string()),
                    TextFont {
                        font_size: 32.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(1.0, 0.0, 0.0)),
                    Transform::from_xyz(0.0, 0.0, 1.1),
                )).id();
                
                commands.entity(building_entity).add_child(indicator);
            }
            (true, Some(&indicator_entity)) => {
                commands.entity(indicator_entity).despawn();
            }
            _ => {}
        }
    }
}