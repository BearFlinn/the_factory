use bevy::{prelude::*, winit::WinitSettings};

use crate::buildings::{BuildingType, TotalProduction};

#[derive(Component)]
pub struct ProductionCounterText;

pub fn setup_production_ui(mut commands: Commands) {
    commands.spawn(Node {
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
            ProductionCounterText,
        ));
    });
}

pub fn update_production_display(
    total_production: Res<TotalProduction>,
    mut text_query: Query<&mut Text, With<ProductionCounterText>>,
) {
    if total_production.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
            **text = format!("Production: {}", total_production.value);
        }
    }
}

#[derive(Component)]
pub struct BuildingButton {
    building_type: BuildingType,
    is_selected: bool,
}

#[derive(Resource, Default)]
pub struct SelectedBuildingType {
    pub building_type: Option<BuildingType>,
}

pub fn setup_building_hotbar(mut commands: Commands) {
    commands.insert_resource(SelectedBuildingType::default());

    commands.spawn((Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        position_type: PositionType::Absolute,
        bottom: Val::Px(20.0),
        justify_self: JustifySelf::Center,
        height: Val::Px(150.0),
        width: Val::Percent(75.0),
        ..default()
    },
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|parent| {
        spawn_building_button(
            parent,
            "Connector\n[1]",
            BuildingType::Connector,
            Color::srgb(0.7, 0.3, 0.7),
        );
        
        spawn_building_button(
            parent,
            "Harvester\n[2]",
            BuildingType::Harvester,
            Color::srgb(0.3, 0.7, 0.3),
        );
        
        spawn_building_button(
            parent,
            "Hub\n[3]",
            BuildingType::Hub,
            Color::srgb(0.3, 0.3, 0.7),
        );
    });
}

fn spawn_building_button(
    parent: &mut ChildBuilder,
    text: &str,
    building_type: BuildingType,
    icon_color: Color,
) {
    parent.spawn((
        Button,
        Node {
            width: Val::Px(100.0),
            height: Val::Px(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        BorderColor(Color::srgb(0.3, 0.3, 0.3)),
        BuildingButton { building_type, is_selected: false },
    ))
    .with_children(|parent| {
        parent.spawn(Node {
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            margin: UiRect::bottom(Val::Px(5.0)),
            ..default()
        })
        .insert(BackgroundColor(icon_color));
        
        parent.spawn((
            Text::new(text),
            TextFont {
                font_size: 16.0,
                ..default()
            },
        ));
    });
}

pub fn handle_building_button_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BuildingButton,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
    mut selected_building: ResMut<SelectedBuildingType>,
) {
    for (interaction, mut building_button, mut bg_color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                selected_building.building_type = Some(building_button.building_type.clone());
                building_button.is_selected = true;
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
                if building_button.is_selected == false {
                    *border_color = BorderColor(Color::srgb(0.5, 0.5, 0.5));
                }
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));
                if building_button.is_selected == false {
                    *border_color = BorderColor(Color::srgb(0.3, 0.3, 0.3));
                }
            }
        }
    }
}

pub fn update_building_button_selection_visual(
    selected_building: Res<SelectedBuildingType>,
    mut button_query: Query<(&mut BuildingButton, &mut BorderColor)>,
) {
    if !selected_building.is_changed() {
        return;
    }
    
    for (mut building_button, mut border_color) in &mut button_query {
        if let Some(selected_type) = &selected_building.building_type {
            if building_button.building_type == *selected_type {
                *border_color = BorderColor(Color::srgb(1.0, 1.0, 1.0));
                println!("Selected building: {:?}", building_button.building_type);
            } else {
                *border_color = BorderColor(Color::srgb(0.3, 0.3, 0.3));
                building_button.is_selected = false;
            }
        } else {
            *border_color = BorderColor(Color::srgb(0.3, 0.3, 0.3));
        }
    }
}

pub fn handle_building_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_building: ResMut<SelectedBuildingType>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        selected_building.building_type = Some(BuildingType::Connector);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        selected_building.building_type = Some(BuildingType::Harvester);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        selected_building.building_type = Some(BuildingType::Hub);
    } else if keyboard.just_pressed(KeyCode::Escape) {
        selected_building.building_type = None;
    }
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, (setup_building_hotbar, setup_production_ui))
            .add_systems(
                Update,
                (
                    update_production_display,
                    handle_building_button_interaction,
                    update_building_button_selection_visual,
                    handle_building_hotkeys,
                ),
            )
            .insert_resource(WinitSettings::desktop_app());
    }
}