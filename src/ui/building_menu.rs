use crate::{
    grid::Position,
    materials::{InputPort, InventoryAccess, OutputPort, RecipeRegistry, StoragePort},
    structures::{Building, RecipeCrafter},
    systems::Operational,
    ui::{
        interaction_handler::{DynamicStyles, InteractiveUI, Selectable},
        SelectionBehavior, UISystemSet,
    },
};
use bevy::prelude::*;

#[derive(Event)]
pub struct BuildingClickEvent {
    pub building_entity: Entity,
    pub world_position: Vec2,
}

#[derive(Event)]
pub struct CloseMenuEvent {
    pub menu_entity: Entity,
}

#[derive(Component)]
pub struct BuildingMenu {
    pub target_building: Entity,
    pub world_position: Vec2,
}

#[derive(Component)]
pub struct MenuCloseButton {
    pub menu_entity: Entity,
}

#[derive(Component)]
pub struct MenuContent {
    pub target_building: Entity,
    pub content_type: ContentType,
    pub last_updated: Option<u32>,
}

#[derive(PartialEq, Clone)]
pub enum ContentType {
    Status,
    Storage,
    Crafting,
}

#[derive(Component)]
pub struct RecipeSelector {
    pub target_building: Entity,
    pub recipe_name: String,
}

#[derive(Event)]
pub struct RecipeChangeEvent {
    pub building_entity: Entity,
    pub recipe_name: String,
}

pub fn detect_building_clicks(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    buildings: Query<(Entity, &Position, &Transform), With<Building>>,
    mut click_events: EventWriter<BuildingClickEvent>,
    ui_interactions: Query<&Interaction, With<Button>>,
) {
    if ui_interactions
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered))
    {
        return;
    }

    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Some(world_pos) = camera
        .viewport_to_world(camera_transform, cursor_pos)
        .ok()
        .map(|ray| ray.origin.truncate())
    else {
        return;
    };

    for (entity, _position, transform) in buildings.iter() {
        let building_world_pos = transform.translation.truncate();
        if world_pos.distance(building_world_pos) < 32.0 {
            click_events.send(BuildingClickEvent {
                building_entity: entity,
                world_position: building_world_pos,
            });
            break;
        }
    }
}

pub fn spawn_building_menu(
    mut commands: Commands,
    mut click_events: EventReader<BuildingClickEvent>,
    existing_menus: Query<(&BuildingMenu, Entity)>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    buildings: Query<&Name, With<Building>>,
) {
    for click in click_events.read() {
        if existing_menus
            .iter()
            .any(|(menu, _)| menu.target_building == click.building_entity)
        {
            continue;
        }

        let Ok((camera, camera_transform)) = camera_q.get_single() else {
            continue;
        };
        let Ok(window) = windows.get_single() else {
            continue;
        };
        let Some(screen_pos) = camera
            .world_to_viewport(camera_transform, click.world_position.extend(0.0))
            .ok()
        else {
            continue;
        };

        let building_name = buildings
            .get(click.building_entity)
            .map(Name::as_str)
            .unwrap_or("Unknown Building");

        let menu_x = (screen_pos.x + 50.0).clamp(10.0, window.width() - 300.0);
        let menu_y = (screen_pos.y - 100.0).clamp(10.0, window.height() - 250.0);

        let menu_entity = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(menu_x),
                    top: Val::Px(menu_y),
                    width: Val::Px(280.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
                BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                BuildingMenu {
                    target_building: click.building_entity,
                    world_position: click.world_position,
                },
            ))
            .id();

        commands.entity(menu_entity).with_children(|parent| {
            spawn_menu_header(parent, building_name, menu_entity);

            spawn_content_section(parent, click.building_entity, ContentType::Status);
            spawn_content_section(parent, click.building_entity, ContentType::Storage);
            spawn_content_section(parent, click.building_entity, ContentType::Crafting);
        });
    }
}

fn spawn_menu_header(parent: &mut ChildBuilder, title: &str, menu_entity: Entity) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(30.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ));

            let close_styles = InteractiveUI::new()
                .default(DynamicStyles::new().with_background(Color::srgb(0.6, 0.2, 0.2)))
                .on_hover(DynamicStyles::new().with_background(Color::srgb(0.8, 0.3, 0.3)));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    close_styles,
                    Selectable::new(),
                    MenuCloseButton { menu_entity },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("x"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        });
}

fn spawn_content_section(
    parent: &mut ChildBuilder,
    building_entity: Entity,
    content_type: ContentType,
) {
    let section_title = match content_type {
        ContentType::Status => "Status",
        ContentType::Storage => "Storage",
        ContentType::Crafting => "Production",
    };

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
            MenuContent {
                target_building: building_entity,
                content_type,
                last_updated: None,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(section_title),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Loading..."),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });
}

pub fn handle_menu_close_buttons_interaction(
    close_buttons: Query<(&MenuCloseButton, &Interaction), Changed<Interaction>>,
    mut close_events: EventWriter<CloseMenuEvent>,
) {
    for (close_button, interaction) in &close_buttons {
        if *interaction == Interaction::Pressed {
            close_events.send(CloseMenuEvent {
                menu_entity: close_button.menu_entity,
            });
        }
    }
}

pub fn process_menu_close_events(
    mut commands: Commands,
    mut close_events: EventReader<CloseMenuEvent>,
    menu_query: Query<Entity, With<BuildingMenu>>,
) {
    for close_event in close_events.read() {
        if menu_query.contains(close_event.menu_entity) {
            commands.entity(close_event.menu_entity).despawn_recursive();
        }
    }
}

pub fn update_menu_positions(
    mut menu_query: Query<(&mut Node, &BuildingMenu)>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
) {
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };
    let Ok(window) = windows.get_single() else {
        return;
    };

    for (mut node, menu) in &mut menu_query {
        if let Ok(screen_pos) =
            camera.world_to_viewport(camera_transform, menu.world_position.extend(0.0))
        {
            let max_x = window.width() - 300.0;
            let max_y = window.height() - 250.0;

            node.left = Val::Px((screen_pos.x + 50.0).clamp(10.0, max_x));
            node.top = Val::Px((screen_pos.y - 100.0).clamp(10.0, max_y));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_menu_content(
    mut content_query: Query<(Entity, &mut MenuContent)>,
    mut commands: Commands,
    children: Query<&Children>,
    buildings_operational: Query<&Operational, With<Building>>,
    buildings_input_port: Query<&InputPort, With<Building>>,
    buildings_output_port: Query<&OutputPort, With<Building>>,
    buildings_storage_port: Query<&StoragePort, With<Building>>,
    buildings_crafting: Query<&RecipeCrafter, With<Building>>,
    recipe_registry: Res<RecipeRegistry>,
) {
    for (content_entity, mut menu_content) in &mut content_query {
        let should_update = match menu_content.content_type {
            ContentType::Status => buildings_operational
                .get(menu_content.target_building)
                .map(simple_hash)
                .map(|hash| menu_content.last_updated != Some(hash))
                .unwrap_or(false),
            ContentType::Storage => {
                let input_hash = buildings_input_port
                    .get(menu_content.target_building)
                    .map(simple_hash);
                let output_hash = buildings_output_port
                    .get(menu_content.target_building)
                    .map(simple_hash);
                let storage_hash = buildings_storage_port
                    .get(menu_content.target_building)
                    .map(simple_hash);

                let combined_hash = input_hash.ok().or(output_hash.ok()).or(storage_hash.ok());

                combined_hash.is_some_and(|hash| menu_content.last_updated != Some(hash))
            }
            ContentType::Crafting => buildings_crafting
                .get(menu_content.target_building)
                .map(hash_crafter_recipe_state)
                .is_ok_and(|hash| menu_content.last_updated != Some(hash)),
        };

        if should_update {
            if let Ok(content_children) = children.get(content_entity) {
                for &child in content_children.iter().skip(1) {
                    commands.entity(child).despawn_recursive();
                }
            }

            commands.entity(content_entity).with_children(|parent| {
                match menu_content.content_type {
                    ContentType::Status => {
                        if let Ok(operational) =
                            buildings_operational.get(menu_content.target_building)
                        {
                            spawn_status_content(parent, operational);
                            menu_content.last_updated = Some(simple_hash(operational));
                        }
                    }
                    ContentType::Storage => {
                        let entity = menu_content.target_building;
                        if let Ok(input_port) = buildings_input_port.get(entity) {
                            let output_port = buildings_output_port.get(entity).ok();
                            spawn_port_inventory_content(
                                parent,
                                Some(input_port),
                                output_port,
                                None,
                            );
                            menu_content.last_updated = Some(simple_hash(input_port));
                        } else if let Ok(output_port) = buildings_output_port.get(entity) {
                            spawn_port_inventory_content(parent, None, Some(output_port), None);
                            menu_content.last_updated = Some(simple_hash(output_port));
                        } else if let Ok(storage_port) = buildings_storage_port.get(entity) {
                            spawn_port_inventory_content(parent, None, None, Some(storage_port));
                            menu_content.last_updated = Some(simple_hash(storage_port));
                        }
                    }
                    ContentType::Crafting => {
                        if let Ok(crafter) = buildings_crafting.get(menu_content.target_building) {
                            spawn_crafting_content(
                                parent,
                                crafter,
                                &recipe_registry,
                                menu_content.target_building,
                            );
                            menu_content.last_updated = Some(hash_crafter_recipe_state(crafter));
                        }
                    }
                }
            });
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn simple_hash<T: std::fmt::Debug>(value: &T) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let debug_string = format!("{value:?}");
    let mut hasher = DefaultHasher::new();
    debug_string.hash(&mut hasher);
    hasher.finish() as u32
}

#[allow(clippy::cast_possible_truncation)]
fn hash_crafter_recipe_state(crafter: &RecipeCrafter) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    crafter.current_recipe.hash(&mut hasher);
    crafter.available_recipes.hash(&mut hasher);
    hasher.finish() as u32
}

fn spawn_status_content(parent: &mut ChildBuilder, operational: &Operational) {
    let is_operational = operational.get_status();
    let status_color = if is_operational {
        Color::srgb(0.2, 0.8, 0.2)
    } else {
        Color::srgb(0.8, 0.2, 0.2)
    };

    parent.spawn((
        Text::new(if is_operational {
            "Operational"
        } else {
            "Not Operational"
        }),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(status_color),
    ));

    if !is_operational {
        if let Some(conditions) = &operational.0 {
            for condition in conditions {
                let condition_text = format!("{condition}");
                if !condition_text.is_empty() {
                    parent.spawn((
                        Text::new(format!("  - {condition_text}")),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.6, 0.4)),
                    ));
                }
            }
        }
    }
}

fn spawn_port_inventory_content(
    parent: &mut ChildBuilder,
    input_port: Option<&InputPort>,
    output_port: Option<&OutputPort>,
    storage_port: Option<&StoragePort>,
) {
    let spawn_port_items =
        |parent: &mut ChildBuilder, label: &str, access: &dyn InventoryAccess, color: Color| {
            parent.spawn((
                Text::new(format!("{label}:")),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(color),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));

            if access.is_empty() {
                parent.spawn((
                    Text::new("  Empty"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                ));
            } else {
                for (item_name, &quantity) in access.items() {
                    parent.spawn((
                        Text::new(format!("  {item_name}: {quantity}")),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));
                }
            }

            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let usage_percent =
                (access.get_total_quantity() as f32 / access.capacity() as f32 * 100.0) as u32;
            parent.spawn((
                Text::new(format!(
                    "  {}/{} ({}%)",
                    access.get_total_quantity(),
                    access.capacity(),
                    usage_percent
                )),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        };

    if let Some(storage) = storage_port {
        spawn_port_items(parent, "Storage", storage, Color::srgb(0.6, 0.8, 0.6));
        return;
    }

    if let Some(input) = input_port {
        spawn_port_items(parent, "Input", input, Color::srgb(0.6, 0.7, 0.9));
    }

    if let Some(output) = output_port {
        spawn_port_items(parent, "Output", output, Color::srgb(0.9, 0.7, 0.6));
    }
}

fn spawn_crafting_content(
    parent: &mut ChildBuilder,
    crafter: &RecipeCrafter,
    recipe_registry: &RecipeRegistry,
    building_entity: Entity,
) {
    if crafter.is_multi_recipe() {
        spawn_recipe_selector(parent, crafter, building_entity);
    }

    if let Some(recipe_name) = crafter.get_active_recipe() {
        parent.spawn((
            Text::new(format!("Recipe: {recipe_name}")),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
        ));

        let progress = crafter.timer.elapsed_secs() / crafter.timer.duration().as_secs_f32();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let progress_percent = (progress * 100.0) as u32;

        parent.spawn((
            Text::new(format!("Progress: {progress_percent}%")),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.9, 0.7)),
        ));

        if let Some(recipe_def) = recipe_registry.get_definition(recipe_name) {
            if !recipe_def.inputs.is_empty() {
                parent.spawn((
                    Text::new("Inputs:"),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                ));
                for (item, quantity) in &recipe_def.inputs {
                    parent.spawn((
                        Text::new(format!("  {quantity} {item}")),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));
                }
            }

            if !recipe_def.outputs.is_empty() {
                parent.spawn((
                    Text::new("Outputs:"),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                ));
                for (item, quantity) in &recipe_def.outputs {
                    parent.spawn((
                        Text::new(format!("  {quantity} {item}")),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));
                }
            }
        }
    } else if crafter.is_multi_recipe() {
        parent.spawn((
            Text::new("No recipe selected"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
        ));
    }
}

fn spawn_recipe_selector(
    parent: &mut ChildBuilder,
    crafter: &RecipeCrafter,
    building_entity: Entity,
) {
    parent.spawn((
        Text::new("Available Recipes:"),
        TextFont {
            font_size: 10.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Node {
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        },
    ));

    for recipe_name in &crafter.available_recipes {
        let is_selected = crafter.get_active_recipe() == Some(recipe_name);

        let button_styles = InteractiveUI::new()
            .default(
                DynamicStyles::new()
                    .with_background(Color::srgb(0.2, 0.2, 0.2))
                    .with_border(Color::srgb(0.4, 0.4, 0.4)),
            )
            .on_hover(
                DynamicStyles::new()
                    .with_background(Color::srgb(0.3, 0.3, 0.3))
                    .with_border(Color::srgb(0.6, 0.6, 0.6)),
            )
            .selected(
                DynamicStyles::new()
                    .with_background(Color::srgb(0.2, 0.4, 0.2))
                    .with_border(Color::srgb(0.4, 0.8, 0.4)),
            );

        parent
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(2.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                button_styles,
                Selectable {
                    is_selected,
                    selection_behavior: SelectionBehavior::Exclusive(format!(
                        "recipe_selector_{}",
                        building_entity.index()
                    )),
                    selection_group: Some(format!("recipe_selector_{}", building_entity.index())),
                },
                RecipeSelector {
                    target_building: building_entity,
                    recipe_name: recipe_name.clone(),
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(recipe_name.clone()),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
            });
    }
}

pub fn handle_escape_close_menus(
    keyboard: Res<ButtonInput<KeyCode>>,
    menu_query: Query<Entity, With<BuildingMenu>>,
    mut close_events: EventWriter<CloseMenuEvent>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        for menu_entity in &menu_query {
            close_events.send(CloseMenuEvent { menu_entity });
        }
    }
}

pub fn handle_recipe_selection(
    recipe_selectors: Query<(Entity, &RecipeSelector, &Selectable), Changed<Selectable>>,
    mut recipe_change_events: EventWriter<RecipeChangeEvent>,
    mut previous_states: Local<std::collections::HashMap<Entity, bool>>,
) {
    for (entity, selector, selectable) in &recipe_selectors {
        let was_selected = previous_states.get(&entity).copied().unwrap_or(false);
        let is_selected = selectable.is_selected;

        if !was_selected && is_selected {
            recipe_change_events.send(RecipeChangeEvent {
                building_entity: selector.target_building,
                recipe_name: selector.recipe_name.clone(),
            });
        }

        previous_states.insert(entity, is_selected);
    }
}

pub fn apply_recipe_changes(
    mut recipe_events: EventReader<RecipeChangeEvent>,
    mut buildings: Query<&mut RecipeCrafter, With<Building>>,
) {
    for event in recipe_events.read() {
        if let Ok(mut crafter) = buildings.get_mut(event.building_entity) {
            if let Err(error) = crafter.set_recipe(event.recipe_name.clone()) {
                warn!(
                    "Failed to set recipe '{}' on building: {}",
                    event.recipe_name, error
                );
            } else {
                info!(
                    "Recipe changed to '{}' for building {:?}",
                    event.recipe_name, event.building_entity
                );
            }
        }
    }
}

pub struct BuildingMenuPlugin;

impl Plugin for BuildingMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BuildingClickEvent>()
            .add_event::<CloseMenuEvent>()
            .add_event::<RecipeChangeEvent>()
            .add_systems(
                Update,
                (
                    (detect_building_clicks, handle_escape_close_menus)
                        .in_set(UISystemSet::InputDetection),
                    (
                        spawn_building_menu,
                        handle_menu_close_buttons_interaction,
                        process_menu_close_events,
                        handle_recipe_selection,
                    )
                        .in_set(UISystemSet::EntityManagement),
                    (
                        update_menu_positions,
                        update_menu_content,
                        apply_recipe_changes,
                    )
                        .in_set(UISystemSet::LayoutUpdates),
                ),
            );
    }
}
