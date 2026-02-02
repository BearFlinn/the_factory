use crate::{
    materials::{
        items::{Cargo, InputPort, OutputPort, StoragePort},
        InventoryAccess, ItemRegistry,
    },
    structures::Building,
    systems::Operational,
    workers::Worker,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct InventoryDisplay;

#[derive(Component)]
pub struct NonOperationalIndicator;

pub fn update_inventory_display(
    mut commands: Commands,
    buildings_and_workers: Query<
        (
            Entity,
            Option<&OutputPort>,
            Option<&InputPort>,
            Option<&StoragePort>,
            Option<&Cargo>,
        ),
        Or<(With<Building>, With<Worker>)>,
    >,
    mut inventory_displays: Query<&mut Text2d, With<InventoryDisplay>>,
    children: Query<&Children>,
    changed_inventories: Query<
        Entity,
        (
            Or<(With<Worker>, With<Building>)>,
            Or<(
                Changed<OutputPort>,
                Changed<InputPort>,
                Changed<StoragePort>,
                Changed<Cargo>,
            )>,
        ),
    >,
    item_registry: Res<ItemRegistry>,
) {
    for (entity, output_port, input_port, storage_port, cargo) in buildings_and_workers.iter() {
        let should_update = changed_inventories.contains(entity);

        let existing_display = children.get(entity).ok().and_then(|children| {
            children
                .iter()
                .find(|&child| inventory_displays.contains(child))
        });

        let items_to_display: Option<std::collections::HashMap<String, u32>> = output_port
            .map(InventoryAccess::get_all_items)
            .or_else(|| input_port.map(InventoryAccess::get_all_items))
            .or_else(|| storage_port.map(InventoryAccess::get_all_items))
            .or_else(|| cargo.map(InventoryAccess::get_all_items));

        let Some(items) = items_to_display else {
            continue;
        };

        let display_text = if items.is_empty() {
            "Empty".to_string()
        } else {
            items
                .iter()
                .map(|(item_name, &quantity)| {
                    let name = item_registry
                        .get_definition(item_name)
                        .map_or("Unknown", |def| def.name.as_str());
                    format!("{name}: {quantity}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        if let Some(display_entity) = existing_display {
            if should_update {
                if let Ok(mut text) = inventory_displays.get_mut(display_entity) {
                    text.0 = display_text;
                }
            }
        } else {
            let display = commands
                .spawn((
                    InventoryDisplay,
                    Text2d::new(display_text),
                    TextFont {
                        font_size: 12.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(0.2, 0.2, 0.2)),
                    Transform::from_xyz(0.0, 30.0, 1.1),
                ))
                .id();

            commands.entity(entity).add_child(display);
        }
    }
}

pub fn update_operational_indicators(
    mut commands: Commands,
    mut buildings: Query<(Entity, &Operational), (With<Building>, Changed<Operational>)>,
    indicators: Query<Entity, With<NonOperationalIndicator>>,
    children: Query<&Children>,
) {
    for (building_entity, operational) in &mut buildings {
        let existing_indicator = children
            .get(building_entity)
            .ok()
            .and_then(|children| children.iter().find(|&child| indicators.contains(child)));

        match (operational.get_status(), existing_indicator) {
            (false, None) => {
                let indicator = commands
                    .spawn((
                        NonOperationalIndicator,
                        Text2d("!".to_string()),
                        TextFont {
                            font_size: 32.0,
                            ..Default::default()
                        },
                        TextColor(Color::srgb(1.0, 0.0, 0.0)),
                        Transform::from_xyz(0.0, 0.0, 1.1),
                    ))
                    .id();

                commands.entity(building_entity).add_child(indicator);
            }
            (true, Some(indicator_entity)) => {
                commands.entity(indicator_entity).despawn();
            }
            _ => {}
        }
    }
}
