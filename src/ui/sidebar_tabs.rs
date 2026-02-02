use bevy::{picking::hover::Hovered, prelude::*, ui::Checked};
use std::collections::HashSet;

use crate::structures::{BuildingCategory, BuildingRegistry};
use crate::ui::style::{ButtonStyle, BUTTON_BG, PANEL_BORDER};
use crate::ui::UISystemSet;

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

    pub fn spawn(&self, parent: &mut ChildSpawnerCommands, registry: &BuildingRegistry) -> Entity {
        let color = get_building_type_color(registry, self.building_type);
        let hotkey = get_building_type_hotkey(self.building_type);

        let tab_button = parent
            .spawn((
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
                BackgroundColor(BUTTON_BG),
                BorderColor::all(PANEL_BORDER),
                ButtonStyle::tab(),
                Hovered::default(),
                SidebarTab {
                    building_type: self.building_type,
                    is_active: self.is_active,
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(color),
                ));

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
    parent: &mut ChildSpawnerCommands,
    registry: &BuildingRegistry,
) -> Entity {
    let available_types = get_available_building_categories(registry);

    let tab_container = parent
        .spawn((
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
                let is_active = index == 0;
                let tab = SidebarTab::new(*building_type, is_active);
                tab.spawn(parent, registry);
            }
        })
        .id();

    tab_container
}

pub fn handle_tab_interactions(
    mut commands: Commands,
    interactions: Query<(Entity, &Interaction), (Changed<Interaction>, With<SidebarTab>)>,
    mut all_tabs: Query<(Entity, &mut SidebarTab)>,
) {
    let mut clicked_entity = None;

    for (entity, interaction) in &interactions {
        if *interaction == Interaction::Pressed {
            clicked_entity = Some(entity);
        }
    }

    if let Some(clicked) = clicked_entity {
        for (entity, mut tab) in &mut all_tabs {
            if entity == clicked {
                tab.set_active(true);
                commands.entity(entity).insert(Checked);
            } else {
                tab.set_active(false);
                commands.entity(entity).remove::<Checked>();
            }
        }
    }
}

pub fn handle_tab_hotkeys(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tab_query: Query<(Entity, &mut SidebarTab)>,
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
        for (entity, mut tab) in &mut tab_query {
            if tab.building_type == building_type {
                commands.entity(entity).insert(Checked);
                tab.set_active(true);
            } else {
                commands.entity(entity).remove::<Checked>();
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
    sorted_types.sort_by_key(|t| format!("{t:?}"));
    sorted_types
}

fn get_building_type_color(
    registry: &BuildingRegistry,
    building_category: BuildingCategory,
) -> Color {
    for building_name in registry.get_all_building_names() {
        if let Some(definition) = registry.get_definition(&building_name) {
            if definition.category == building_category {
                return Color::srgb(
                    definition.appearance.color.0,
                    definition.appearance.color.1,
                    definition.appearance.color.2,
                );
            }
        }
    }
    Color::srgb(0.5, 0.5, 0.5)
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
        app.add_systems(
            Update,
            (
                handle_tab_hotkeys.in_set(UISystemSet::InputDetection),
                handle_tab_interactions.in_set(UISystemSet::VisualUpdates),
            ),
        );
    }
}
