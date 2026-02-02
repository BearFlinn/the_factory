use bevy::prelude::*;

use crate::{
    materials::{InventoryAccess, ItemRegistry, StoragePort},
    structures::Hub,
    systems::{ComputeGrid, PowerGrid},
    ui::style::{PANEL_BG, TEXT_COLOR},
};

#[derive(Component)]
pub struct ProductionText;

#[derive(Component)]
pub struct PowerGridText;

#[derive(Component)]
pub struct ComputeGridText;

pub fn setup_production_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Total Production: 0"),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(TEXT_COLOR),
                ProductionText,
            ));

            parent.spawn((
                Text::new("Available Power: 0"),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(TEXT_COLOR),
                PowerGridText,
            ));

            parent.spawn((
                Text::new("Available Compute: 0"),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(TEXT_COLOR),
                ComputeGridText,
            ));
        });
}

pub fn update_compute_grid_text(
    compute_grid: Res<ComputeGrid>,
    mut text_query: Query<&mut Text, With<ComputeGridText>>,
) {
    if compute_grid.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            **text = format!("Available Compute: {}", compute_grid.available);
        }
    }
}

pub fn update_power_grid_text(
    power_grid: Res<PowerGrid>,
    mut text_query: Query<&mut Text, With<PowerGridText>>,
) {
    if power_grid.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            **text = format!("Available Power: {}", power_grid.available);
        }
    }
}

pub fn update_production_text(
    central_storage_port: Query<&StoragePort, (With<Hub>, Changed<StoragePort>)>,
    mut text_query: Query<&mut Text, With<ProductionText>>,
    item_registry: Res<ItemRegistry>,
) {
    let Ok(storage_port) = central_storage_port.single() else {
        return;
    };

    let items = storage_port.items();

    if let Ok(mut text) = text_query.single_mut() {
        if items.is_empty() {
            **text = "Central Storage: Empty".to_string();
        } else {
            let items_text = items
                .iter()
                .map(|(item_name, &quantity)| {
                    let name = item_registry
                        .get_definition(item_name)
                        .map_or("Unknown", |def| def.name.as_str());
                    format!("{name}: {quantity}")
                })
                .collect::<Vec<_>>()
                .join(",\n");

            **text = format!("Central Storage:\n{items_text}");
        }
    }
}

pub struct ProductionDisplayPlugin;

impl Plugin for ProductionDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_production_ui);
        app.add_systems(
            Update,
            (
                update_production_text,
                update_power_grid_text,
                update_compute_grid_text,
            ),
        );
    }
}
