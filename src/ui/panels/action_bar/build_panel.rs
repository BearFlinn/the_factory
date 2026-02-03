use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Checked;
use std::collections::HashSet;

use crate::{
    structures::{BuildingCategory, BuildingRegistry},
    ui::{
        icons::IconAtlas,
        popups::tooltip::TooltipTarget,
        scroll::Scrollable,
        style::{
            ButtonStyle, ACTION_BAR_WIDTH, BUTTON_BG, CANCEL_BG, DIM_TEXT, HEADER_COLOR, PANEL_BG,
            PANEL_BORDER, TEXT_COLOR, TOP_BAR_HEIGHT,
        },
        UISystemSet,
    },
};

#[derive(Resource, Default)]
pub struct SelectedBuilding {
    pub building_name: Option<String>,
}

#[derive(Component)]
pub struct BuildPanel;

#[derive(Component)]
pub struct BuildPanelContent;

#[derive(Component)]
pub struct BuildingButton {
    pub building_name: String,
    pub is_selected: bool,
}

#[derive(Component)]
pub struct BuildPanelTab {
    pub building_type: BuildingCategory,
    pub is_active: bool,
}

#[derive(Component)]
pub struct BuildPanelCloseButton;

impl BuildingButton {
    pub fn new(building_name: String) -> Self {
        Self {
            building_name,
            is_selected: false,
        }
    }
}

pub fn spawn_build_panel(
    commands: &mut Commands,
    registry: &BuildingRegistry,
    _icon_atlas: &IconAtlas,
) {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ACTION_BAR_WIDTH + 4.0),
                top: Val::Px(TOP_BAR_HEIGHT + 4.0),
                width: Val::Px(280.0),
                max_height: Val::Vh(80.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(PANEL_BORDER),
            Interaction::None,
            BuildPanel,
        ))
        .id();

    let header = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        })
        .id();

    let title = commands
        .spawn((
            Text::new("Build"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(HEADER_COLOR),
        ))
        .id();

    let close_btn = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CANCEL_BG),
            ButtonStyle::close(),
            Hovered::default(),
            BuildPanelCloseButton,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new("x"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
        })
        .id();

    commands.entity(header).add_children(&[title, close_btn]);

    let tab_container = spawn_build_tabs(commands, registry);

    let content = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            Scrollable,
            BuildPanelContent,
        ))
        .id();

    commands
        .entity(panel)
        .add_children(&[header, tab_container, content]);
}

pub fn despawn_build_panel(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).despawn();
}

fn spawn_build_tabs(commands: &mut Commands, registry: &BuildingRegistry) -> Entity {
    let available = get_available_building_categories(registry);

    let container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(36.0),
            flex_direction: FlexDirection::Row,
            margin: UiRect::bottom(Val::Px(4.0)),
            column_gap: Val::Px(2.0),
            ..default()
        })
        .id();

    for (index, &building_type) in available.iter().enumerate() {
        let is_active = index == 0;
        let color = get_building_type_color(registry, building_type);
        let hotkey = get_building_type_hotkey(building_type);

        let mut tab_cmd = commands.spawn((
            Button,
            Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                column_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(BUTTON_BG),
            BorderColor::all(PANEL_BORDER),
            ButtonStyle::tab(),
            Hovered::default(),
            BuildPanelTab {
                building_type,
                is_active,
            },
        ));

        if is_active {
            tab_cmd.insert(Checked);
        }

        let tab_entity = tab_cmd
            .with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(color),
                ));

                parent.spawn((
                    Text::new(hotkey.to_string()),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
            })
            .id();

        commands.entity(container).add_child(tab_entity);
    }

    container
}

pub fn handle_build_panel_close(
    close_buttons: Query<&Interaction, (Changed<Interaction>, With<BuildPanelCloseButton>)>,
    mut active_panel: ResMut<super::ActivePanel>,
) {
    for interaction in &close_buttons {
        if *interaction == Interaction::Pressed {
            *active_panel = super::ActivePanel::None;
        }
    }
}

pub fn handle_tab_interactions(
    mut commands: Commands,
    interactions: Query<(Entity, &Interaction), (Changed<Interaction>, With<BuildPanelTab>)>,
    mut all_tabs: Query<(Entity, &mut BuildPanelTab)>,
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
                tab.is_active = true;
                commands.entity(entity).insert(Checked);
            } else {
                tab.is_active = false;
                commands.entity(entity).remove::<Checked>();
            }
        }
    }
}

pub fn handle_tab_hotkeys(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tab_query: Query<(Entity, &mut BuildPanelTab)>,
    active_panel: Res<super::ActivePanel>,
) {
    if *active_panel != super::ActivePanel::Build {
        return;
    }

    let target = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(BuildingCategory::Logistics)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(BuildingCategory::Production)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(BuildingCategory::Utility)
    } else {
        None
    };

    if let Some(building_type) = target {
        for (entity, mut tab) in &mut tab_query {
            if tab.building_type == building_type {
                commands.entity(entity).insert(Checked);
                tab.is_active = true;
            } else {
                commands.entity(entity).remove::<Checked>();
                tab.is_active = false;
            }
        }
    }
}

pub fn update_building_buttons_on_tab_change(
    mut commands: Commands,
    tab_query: Query<&BuildPanelTab, Changed<BuildPanelTab>>,
    all_tabs_query: Query<&BuildPanelTab>,
    content_query: Query<Entity, With<BuildPanelContent>>,
    existing_buttons: Query<Entity, With<BuildingButton>>,
    registry: Res<BuildingRegistry>,
) {
    if tab_query.is_empty() {
        return;
    }

    let active_tab_type = all_tabs_query
        .iter()
        .find(|tab| tab.is_active)
        .map(|tab| tab.building_type);

    if let Ok(content_entity) = content_query.single() {
        for entity in existing_buttons.iter() {
            commands.entity(entity).despawn();
        }

        if let Some(building_category) = active_tab_type {
            commands.entity(content_entity).with_children(|parent| {
                spawn_building_buttons_for_category(parent, building_category, &registry);
            });
        }
    }
}

pub fn handle_building_button_interactions(
    mut commands: Commands,
    button_query: Query<
        (Entity, &BuildingButton, &Interaction),
        (Changed<Interaction>, With<BuildingButton>),
    >,
    checked_buttons: Query<Entity, (With<BuildingButton>, With<Checked>)>,
    mut selected_building: ResMut<SelectedBuilding>,
) {
    for (entity, button, interaction) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        for other in &checked_buttons {
            commands.entity(other).remove::<Checked>();
        }

        commands.entity(entity).insert(Checked);
        selected_building.building_name = Some(button.building_name.clone());
    }
}

fn spawn_building_buttons_for_category(
    parent: &mut ChildSpawnerCommands,
    building_category: BuildingCategory,
    registry: &BuildingRegistry,
) {
    let buildings = registry.get_buildings_by_category(building_category);

    for building_name in buildings {
        if let Some(definition) = registry.get_definition(&building_name) {
            let button = BuildingButton::new(building_name);

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(50.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON_BG),
                    BorderColor::all(PANEL_BORDER),
                    ButtonStyle::building_button(),
                    Hovered::default(),
                    button,
                    TooltipTarget,
                ))
                .with_children(|btn| {
                    btn.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: Val::Px(30.0),
                                height: Val::Px(30.0),
                                margin: UiRect::right(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(
                                definition.appearance.color.0,
                                definition.appearance.color.1,
                                definition.appearance.color.2,
                                1.0,
                            )),
                        ));

                        row.spawn((
                            Text::new(&definition.name),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                        ));
                    });

                    btn.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|cost_col| {
                        let cost_text = format_cost_display(&definition.placement.cost.inputs);
                        cost_col.spawn((
                            Text::new(cost_text),
                            TextFont {
                                font_size: 9.0,
                                ..default()
                            },
                            TextColor(DIM_TEXT),
                        ));
                    });
                });
        }
    }
}

fn format_cost_display(inputs: &std::collections::HashMap<String, u32>) -> String {
    if inputs.is_empty() {
        return "Free".to_string();
    }

    let mut sorted_inputs: Vec<_> = inputs.iter().collect();
    sorted_inputs.sort_by_key(|(name, _)| name.as_str());

    if sorted_inputs.len() <= 3 {
        sorted_inputs
            .iter()
            .map(|(name, quantity)| format!("{quantity} {name}"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let first_two: Vec<String> = sorted_inputs
            .iter()
            .take(2)
            .map(|(name, quantity)| format!("{quantity} {name}"))
            .collect();

        format!("{}\n...", first_two.join("\n"))
    }
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

pub struct BuildPanelPlugin;

impl Plugin for BuildPanelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectedBuilding::default())
            .add_systems(
                Update,
                (
                    handle_tab_hotkeys.in_set(UISystemSet::InputDetection),
                    (
                        handle_build_panel_close,
                        update_building_buttons_on_tab_change,
                    )
                        .in_set(UISystemSet::EntityManagement),
                    (
                        handle_tab_interactions,
                        handle_building_button_interactions
                            .run_if(not(in_state(crate::ui::UiMode::WorkflowCreate))),
                    )
                        .in_set(UISystemSet::VisualUpdates),
                ),
            );
    }
}
