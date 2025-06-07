use bevy::prelude::*;
use crate::ui::interaction_handler::{Selectable, InteractiveUI, DynamicStyles, SelectionBehavior};
use crate::structures::{BuildingCategory, BuildingRegistry};

#[derive(Resource, Default)]
pub struct SelectedBuilding {
    // TODO remove option
    pub building_name: Option<String>,
}

#[derive(Component)]
pub struct BuildingButton {
    pub building_name: String,
    pub is_selected: bool,
}

impl BuildingButton {
    pub fn new(building_name: String) -> Self {
        Self {
            building_name,
            is_selected: false,
        }
    }

    pub fn spawn(&self, parent: &mut ChildBuilder, registry: &BuildingRegistry) -> Entity {
        let definition = registry.get_definition(&self.building_name).unwrap();
        // Define styles for the building button
        let button_styles = InteractiveUI::new()
            .default(DynamicStyles::new()
                .with_background(Color::srgb(0.2, 0.2, 0.2))
                .with_border(Color::srgb(0.4, 0.4, 0.4)))
            .on_hover(DynamicStyles::new()
                .with_background(Color::srgb(0.3, 0.3, 0.3))
                .with_border(Color::srgb(0.6, 0.6, 0.6)))
            .selected(DynamicStyles::new()
                .with_background(Color::srgb(0.3, 0.4, 0.2))
                .with_border(Color::srgb(0.6, 0.8, 0.4)));

        // Create the main building button
        let building_button = parent.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(5.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            button_styles,
            Selectable::new()
                .with_behavior(SelectionBehavior::Exclusive("building_buttons".to_string()))
                .with_group("building_buttons".to_string()),
            BuildingButton {
                building_name: self.building_name.clone(),
                is_selected: self.is_selected,
            },
        ))
        .with_children(|parent| {
            // Building icon
            parent.spawn((
                Node {
                    width: Val::Px(40.0),
                    height: Val::Px(40.0),
                    margin: UiRect::right(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(definition.appearance.color.0, definition.appearance.color.1, definition.appearance.color.2, 1.0)),
            ));
            
            // Building info container
            parent.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|parent| {
                // Building name
                parent.spawn((
                    Text::new(&definition.name),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                ));
            });
        })
        .id();

        building_button
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }
}

pub fn spawn_building_buttons_for_category(
    parent: &mut ChildBuilder,
    building_category: BuildingCategory,
    registry: &BuildingRegistry,
) {
    let buildings = registry.get_buildings_by_category(building_category);
    
    for building_name in buildings {
        if let Some(_definition) = registry.get_definition(&building_name) {
            let button = BuildingButton::new(building_name);
            button.spawn(parent, registry);
        }
    }
}

pub fn handle_building_button_interactions(
    mut button_query: Query<(&mut BuildingButton, &Selectable), Changed<Selectable>>,
    mut selected_building: ResMut<SelectedBuilding>,
) {
    for (mut button, selectable) in &mut button_query {
        if selectable.is_selected && !button.is_selected {
            button.set_selected(true);
            selected_building.building_name = Some(button.building_name.clone());
            println!("Selected building: {}", button.building_name);
        }
        
        // Update selected state based on selection
        button.is_selected = selectable.is_selected;
        
        // If this button was deselected, clear the resource if it was this building
        if !selectable.is_selected && button.is_selected {
            if selected_building.building_name.as_ref() == Some(&button.building_name) {
                selected_building.building_name = None;
            }
        }
    }
}

pub fn handle_building_selection_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut button_query: Query<(&mut BuildingButton, &mut Selectable)>,
    mut selected_building: ResMut<SelectedBuilding>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        // Clear all building selections
        for (mut button, mut selectable) in &mut button_query {
            if button.is_selected {
                button.set_selected(false);
                selectable.is_selected = false;
            }
        }
        selected_building.building_name = None;
        println!("Cleared building selection");
    }
}

pub fn update_building_buttons_for_active_tab(
    commands: &mut Commands,
    active_building_type: Option<BuildingCategory>,
    content_container: Entity,
    registry: &BuildingRegistry,
    existing_buttons: Query<Entity, With<BuildingButton>>,
) {
    // Clear existing buttons
    for entity in existing_buttons.iter() {
        commands.entity(entity).despawn_recursive();
    }
    
    // Spawn new buttons for the active tab
    if let Some(building_category) = active_building_type {
        commands.entity(content_container).with_children(|parent| {
            spawn_building_buttons_for_category(parent, building_category, registry);
        });
    }
}

#[allow(dead_code)] // Is use in spawn_building_buttons_for_category rust analyzer broky
fn get_buildings_of_category(registry: &BuildingRegistry, building_category: BuildingCategory) -> Vec<String> {
    let mut buildings = Vec::new();
    
    for building_name in registry.get_all_building_names() {
        if let Some(definition) = registry.get_definition(&building_name) {
            if definition.category == building_category {
                buildings.push(building_name);
            }
        }
    }
    
    buildings.sort();
    buildings
}

pub struct BuildingButtonsPlugin;

impl Plugin for BuildingButtonsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectedBuilding::default())
           .add_systems(Update, (
               handle_building_button_interactions,
               handle_building_selection_hotkeys,
           ));
    }
}
