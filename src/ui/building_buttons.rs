use bevy::{picking::hover::Hovered, prelude::*, ui::Checked};

use crate::structures::{BuildingCategory, BuildingRegistry};
use crate::ui::style::{ButtonStyle, BUTTON_BG, DIM_TEXT, PANEL_BORDER};
use crate::ui::{TooltipTarget, UISystemSet};

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

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
    }

    pub fn spawn(&self, parent: &mut ChildSpawnerCommands, registry: &BuildingRegistry) -> Entity {
        let Some(definition) = registry.get_definition(&self.building_name) else {
            warn!("Building definition not found: {}", self.building_name);
            return parent.spawn(Node::default()).id();
        };

        let building_button = parent
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(60.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(8.0)),
                    margin: UiRect::bottom(Val::Px(5.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(BUTTON_BG),
                BorderColor::all(PANEL_BORDER),
                ButtonStyle::building_button(),
                Hovered::default(),
                BuildingButton {
                    building_name: self.building_name.clone(),
                    is_selected: self.is_selected,
                },
                TooltipTarget,
            ))
            .with_children(|parent| {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                width: Val::Px(40.0),
                                height: Val::Px(40.0),
                                margin: UiRect::right(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(
                                definition.appearance.color.0,
                                definition.appearance.color.1,
                                definition.appearance.color.2,
                                1.0,
                            )),
                        ));

                        parent.spawn((
                            Text::new(&definition.name),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                        ));
                    });

                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        justify_content: JustifyContent::Center,
                        ..default()
                    })
                    .with_children(|parent| {
                        let cost_text = format_cost_display(&definition.placement.cost.inputs);
                        parent.spawn((
                            Text::new(cost_text),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(DIM_TEXT),
                        ));
                    });
            })
            .id();

        building_button
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

pub fn spawn_building_buttons_for_category(
    parent: &mut ChildSpawnerCommands,
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

pub fn update_building_buttons_for_active_tab(
    commands: &mut Commands,
    active_building_type: Option<BuildingCategory>,
    content_container: Entity,
    registry: &BuildingRegistry,
    existing_buttons: Query<Entity, With<BuildingButton>>,
) {
    for entity in existing_buttons.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(building_category) = active_building_type {
        commands.entity(content_container).with_children(|parent| {
            spawn_building_buttons_for_category(parent, building_category, registry);
        });
    }
}

#[allow(dead_code)] // Is use in spawn_building_buttons_for_category rust analyzer broky
fn get_buildings_of_category(
    registry: &BuildingRegistry,
    building_category: BuildingCategory,
) -> Vec<String> {
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
            .add_systems(
                Update,
                handle_building_button_interactions
                    .in_set(UISystemSet::VisualUpdates)
                    .run_if(not(in_state(crate::ui::UiMode::WorkflowCreate))),
            );
    }
}
