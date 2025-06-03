use bevy::prelude::*;

use crate::structures::{PowerGrid, TotalProduction};

#[derive(Component)]
pub struct ProductionText;

#[derive(Component)]
pub struct PowerGridText;

pub fn setup_production_ui(mut commands: Commands) {
    commands.spawn(Node {
        flex_direction: FlexDirection::Column,
        position_type: PositionType::Absolute,
        left: Val::Px(20.0),
        top: Val::Px(20.0),
        ..default()
    }).with_children(|parent| {
        parent.spawn((
            Text::new("Total Production: 0"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            ProductionText,
        ));

        parent.spawn((
            Text::new("Power Stats\nProduction: 0\nPower Consumption: 0\nAvailable Power: 0"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            PowerGridText,
        ));
    });
}

pub fn update_power_grid_text(
    power_grid: Res<PowerGrid>,
    mut text_query: Query<&mut Text, With<PowerGridText>>,
) {
    if power_grid.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
            **text = format!("Power Stats\nProduction: {}\nPower Consumption: {}\nAvailable Power: {}", power_grid.capacity, power_grid.usage, power_grid.available);
        }
    }
}

pub fn update_production_text(
    total_production: Res<TotalProduction>,
    mut text_query: Query<&mut Text, With<ProductionText>>,
) {
    if total_production.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
            **text = format!("Total Production: {}", total_production.ore);
        }
    }
}

pub struct ProductionDisplayPlugin;

impl Plugin for ProductionDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_production_ui);
        app.add_systems(Update, (
            update_production_text, 
            update_power_grid_text
        ));
    }
}