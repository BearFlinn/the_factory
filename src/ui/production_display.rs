use bevy::prelude::*;

use crate::{
    materials::{items::Inventory, ItemRegistry},
    structures::Hub,
    systems::{ComputeGrid, PowerGrid},
};

#[derive(Component)]
pub struct ProductionText;

#[derive(Component)]
pub struct PowerGridText;

#[derive(Component)]
pub struct ComputeGridText;

pub fn setup_production_ui(mut commands: Commands) {
    commands.spawn((Node {
        flex_direction: FlexDirection::Column,
        position_type: PositionType::Absolute,
        left: Val::Px(20.0),
        top: Val::Px(20.0),
        ..default()
    },
    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Total Production: 0"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            ProductionText,
        ));

        parent.spawn((
            Text::new("Available Power: 0"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            PowerGridText,
        ));

        parent.spawn((
            Text::new("Available Compute: 0"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            ComputeGridText,
        ));
    });
}

pub fn update_compute_grid_text(
    compute_grid: Res<ComputeGrid>,
    mut text_query: Query<&mut Text, With<ComputeGridText>>,
) {
    if compute_grid.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
            **text = format!("Available Compute: {}", compute_grid.available);
        }
    }
}

pub fn update_power_grid_text(
    power_grid: Res<PowerGrid>,
    mut text_query: Query<&mut Text, With<PowerGridText>>,
) {
    if power_grid.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
            **text = format!("Available Power: {}", power_grid.available);
        }
    }
}

pub fn update_production_text(
    central_inventory: Query<&Inventory, (With<Hub>, Changed<Inventory>)>,
    mut text_query: Query<&mut Text, With<ProductionText>>,
    item_registry: Res<ItemRegistry>,
) {
    if let Ok(inventory) = central_inventory.get_single() {
        if let Ok(mut text) = text_query.get_single_mut() {
            if inventory.items.is_empty() {
                **text = "Central Storage: Empty".to_string();
            } else {
                let items_text = inventory.items.iter()
                    .map(|(item_name, &quantity)| {
                        let name = item_registry.get_definition(&item_name)
                            .map(|def| def.name.as_str())
                            .unwrap_or("Unknown");
                        format!("{}: {}", name, quantity)
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                
                **text = format!("Central Storage:\n{}", items_text);
            }
        }
    }
}

pub struct ProductionDisplayPlugin;

impl Plugin for ProductionDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_production_ui);
        app.add_systems(Update, (
            update_production_text, 
            update_power_grid_text, 
            update_compute_grid_text
        ));
    }
}