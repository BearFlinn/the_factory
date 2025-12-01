use bevy::prelude::*;
use crate::ui::interaction_handler::{Selectable, InteractiveUI, DynamicStyles, SelectionBehavior};
use crate::structures::{BuildingRegistry, BuildingCategory};
use crate::ui::UISystemSet;
use std::collections::HashSet;

#[derive(Component)]
pub struct SidebarTab {
    pub building_type: BuildingCategory,
    pub is_active: bool,
}

#[derive(Component)]
pub struct SidebarTabContainer;

impl SidebarTab {
    pub fn new(building_type: BuildingCategory, is_active: bool) -> Self {
        Self {
            building_type,
            is_active,
        }
    }

    pub fn spawn(&self, parent: &mut ChildBuilder, registry: &BuildingRegistry) -> Entity {
        let color = get_building_type_color(registry, self.building_type);
        let hotkey = get_building_type_hotkey(self.building_type);

        // Define styles for the tab
        let tab_styles = InteractiveUI::new()
            .default(DynamicStyles::new()
                .with_background(Color::srgb(0.15, 0.15, 0.15))
                .with_border(Color::srgb(0.3, 0.3, 0.3)))
            .on_hover(DynamicStyles::new()
                .with_background(Color::srgb(0.25, 0.25, 0.25))
                .with_border(Color::srgb(0.4, 0.4, 0.4)))
            .selected(DynamicStyles::new()
                .with_background(Color::srgb(0.2, 0.3, 0.2))
                .with_border(Color::srgb(0.4, 0.6, 0.4)));

        // Create the main tab button with children
        let tab_button = parent.spawn((
            Button,
            Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            tab_styles,
            Selectable::new()
                .with_behavior(SelectionBehavior::Exclusive("sidebar_tabs".to_string()))
                .with_group("sidebar_tabs".to_string()),
            SidebarTab {
                building_type: self.building_type,
                is_active: self.is_active,
            },
        ))
        .with_children(|parent| {
            // Create the color indicator
            parent.spawn((
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    margin: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(color),
            ));

            // Create the tab label text
            parent.spawn((
                Text::new(format!("{:?}\n{}", self.building_type, hotkey)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
        })
        .id();

        tab_button
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }
}

pub fn spawn_sidebar_tabs(
    parent: &mut ChildBuilder,
    registry: &BuildingRegistry,
) -> Entity {
    let available_types = get_available_building_categories(registry);

    // Create the tab container with tabs as children
    let tab_container = parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(50.0),
            flex_direction: FlexDirection::Row,
            ..default()
        },
        SidebarTabContainer,
    ))
    .with_children(|parent| {
        for (index, building_type) in available_types.iter().enumerate() {
            let is_active = index == 0; // First tab is active by default
            let tab = SidebarTab::new(*building_type, is_active);
            tab.spawn(parent, registry);
        }
    })
    .id();

    tab_container
}

pub fn handle_tab_interactions(
    mut tab_query: Query<(&mut SidebarTab, &Selectable), Changed<Selectable>>,
) {
    // Check if any tab was selected
    for (mut tab, selectable) in &mut tab_query {
        if selectable.is_selected && !tab.is_active {
            tab.set_active(true);
            
            // The InteractiveUI system with Exclusive selection behavior
            // will automatically handle deselecting other tabs in the group
        }
        
        // Update active state based on selection
        tab.is_active = selectable.is_selected;
    }
}

pub fn handle_tab_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tab_query: Query<(&mut SidebarTab, &mut Selectable)>,
) {
    let mut target_building_type = None;

    if keyboard.just_pressed(KeyCode::Digit1) {
        target_building_type = Some(BuildingCategory::Logistics);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        target_building_type = Some(BuildingCategory::Production);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        target_building_type = Some(BuildingCategory::Utility);
    }

    if let Some(building_type) = target_building_type {
        // Find and select the matching tab
        for (mut tab, mut selectable) in &mut tab_query {
            if tab.building_type == building_type {
                selectable.is_selected = true;
                tab.set_active(true);
            } else {
                selectable.is_selected = false;
                tab.set_active(false);
            }
        }
    }
}

pub fn get_active_tab_type(tab_query: &Query<&SidebarTab>) -> Option<BuildingCategory> {
    for tab in tab_query.iter() {
        if tab.is_active {
            return Some(tab.building_type);
        }
    }
    None
}

fn get_available_building_categories(registry: &BuildingRegistry) -> Vec<BuildingCategory> {
    let mut types = HashSet::new();
    
    for building_name in registry.get_all_building_names() {
        if let Some(definition) = registry.get_definition(&building_name) {
            types.insert(definition.category);
        }
    }
    
    let mut sorted_types: Vec<BuildingCategory> = types.into_iter().collect();
    sorted_types.sort_by_key(|t| format!("{:?}", t));
    sorted_types
}

fn get_building_type_color(registry: &BuildingRegistry, building_category: BuildingCategory) -> Color {
    for building_name in registry.get_all_building_names() {
        if let Some(definition) = registry.get_definition(&building_name) {
            if definition.category == building_category {
                return Color::srgb(definition.appearance.color.0, definition.appearance.color.1, definition.appearance.color.2);
            }
        }
    }
    Color::srgb(0.5, 0.5, 0.5) // Default gray if not found
}

fn get_building_type_hotkey(building_type: BuildingCategory) -> &'static str {
    match building_type {
        BuildingCategory::Logistics => "[1]",
        BuildingCategory::Production => "[2]", 
        BuildingCategory::Utility => "[3]",
    }
}

pub struct SidebarTabsPlugin;

impl Plugin for SidebarTabsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_tab_hotkeys.in_set(UISystemSet::InputDetection),
            handle_tab_interactions.in_set(UISystemSet::VisualUpdates),
        ));
    }
}