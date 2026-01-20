use bevy::prelude::*;

use crate::{
    structures::BuildingRegistry,
    ui::{
        get_active_tab_type,
        interaction_handler::{DynamicStyles, InteractiveUI, Selectable},
        spawn_sidebar_tabs, update_building_buttons_for_active_tab, BuildingButton, SidebarTab,
        UISystemSet,
    },
};

#[derive(Component)]
pub struct Sidebar {
    pub is_visible: bool,
}

#[derive(Component)]
pub struct SidebarCloseButton;

#[derive(Component)]
pub struct SidebarToggleButton;

#[derive(Component)]
pub struct SidebarContainer;

#[derive(Component)]
pub struct SidebarContent;

impl Sidebar {
    pub fn new() -> Self {
        Self { is_visible: true }
    }

    #[allow(clippy::too_many_lines)]
    pub fn spawn(&self, commands: &mut Commands, registry: &BuildingRegistry) -> Entity {
        let close_button_styles = InteractiveUI::new()
            .default(DynamicStyles::new().with_background(Color::srgb(0.4, 0.2, 0.2)))
            .on_hover(DynamicStyles::new().with_background(Color::srgb(0.6, 0.3, 0.3)))
            .on_click(DynamicStyles::new().with_background(Color::srgb(0.5, 0.25, 0.25)));

        let toggle_button_styles = InteractiveUI::new()
            .default(DynamicStyles::new().with_background(Color::srgb(0.2, 0.2, 0.2)))
            .on_hover(DynamicStyles::new().with_background(Color::srgb(0.3, 0.3, 0.3)))
            .on_click(DynamicStyles::new().with_background(Color::srgb(0.25, 0.25, 0.25)));

        let sidebar_container = commands
            .spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(400.0),
                    position_type: PositionType::Absolute,
                    align_self: AlignSelf::Center,
                    left: Val::Px(10.0),
                    flex_direction: FlexDirection::Column,
                    display: if self.is_visible {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                SidebarContainer,
                Sidebar {
                    is_visible: self.is_visible,
                },
            ))
            .id();

        let header = commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            })
            .insert(BorderColor(Color::srgb(0.3, 0.3, 0.3)))
            .id();

        let title_text = commands
            .spawn((
                Text::new("Buildings"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
            ))
            .id();

        // Create the close button
        let close_button = commands
            .spawn((
                Button,
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                close_button_styles,
                Selectable::new(),
                SidebarCloseButton,
            ))
            .id();

        let close_button_text = commands
            .spawn((
                Text::new("x"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ))
            .id();

        let content_area = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                SidebarContent,
            ))
            .id();

        let toggle_button = commands
            .spawn((
                Button,
                Node {
                    width: Val::Px(50.0),
                    height: Val::Px(30.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                toggle_button_styles,
                Selectable::new(),
                SidebarToggleButton,
            ))
            .id();

        let toggle_button_text = commands
            .spawn((
                Text::new(if self.is_visible { "▼" } else { "▲" }),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ))
            .id();

        commands.entity(close_button).add_child(close_button_text);
        commands.entity(toggle_button).add_child(toggle_button_text);
        commands.entity(header).add_child(title_text);
        commands.entity(header).add_child(close_button);
        commands.entity(sidebar_container).add_child(header);

        commands.entity(sidebar_container).with_children(|parent| {
            spawn_sidebar_tabs(parent, registry);
        });

        commands.entity(sidebar_container).add_child(content_area);

        sidebar_container
    }

    pub fn toggle_visibility(&mut self) {
        self.is_visible = !self.is_visible;
    }

    pub fn set_visibility(&mut self, visible: bool) {
        self.is_visible = visible;
    }
}

pub fn handle_sidebar_interactions(
    close_button_query: Query<&Selectable, (With<SidebarCloseButton>, Changed<Selectable>)>,
    toggle_button_query: Query<&Selectable, (With<SidebarToggleButton>, Changed<Selectable>)>,
    mut sidebar_query: Query<(&mut Sidebar, &mut Node), With<SidebarContainer>>,
    mut toggle_text_query: Query<&mut Text, With<SidebarToggleButton>>,
) {
    for selectable in &close_button_query {
        if selectable.is_selected {
            for (mut sidebar, mut node) in &mut sidebar_query {
                sidebar.set_visibility(false);
                node.display = Display::None;
            }

            for mut text in &mut toggle_text_query {
                **text = "x".to_string();
            }
        }
    }

    for selectable in &toggle_button_query {
        if selectable.is_selected {
            for (mut sidebar, mut node) in &mut sidebar_query {
                sidebar.toggle_visibility();
                node.display = if sidebar.is_visible {
                    Display::Flex
                } else {
                    Display::None
                };
            }

            for mut text in &mut toggle_text_query {
                let sidebar_visible = sidebar_query.iter().any(|(sidebar, _)| sidebar.is_visible);
                **text = if sidebar_visible { ">" } else { "<" }.to_string();
            }
        }
    }
}

pub fn handle_sidebar_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sidebar_query: Query<(&mut Sidebar, &mut Node), With<SidebarContainer>>,
    mut toggle_text_query: Query<&mut Text, With<SidebarToggleButton>>,
) {
    if keyboard.just_pressed(KeyCode::KeyB) {
        for (mut sidebar, mut node) in &mut sidebar_query {
            sidebar.toggle_visibility();
            node.display = if sidebar.is_visible {
                Display::Flex
            } else {
                Display::None
            };
        }

        for mut text in &mut toggle_text_query {
            let sidebar_visible = sidebar_query.iter().any(|(sidebar, _)| sidebar.is_visible);
            **text = if sidebar_visible { ">" } else { "<" }.to_string();
        }
    }
}

pub fn update_building_buttons_on_tab_change(
    mut commands: Commands,
    tab_query: Query<&SidebarTab, Changed<SidebarTab>>,
    all_tabs_query: Query<&SidebarTab>,
    content_query: Query<Entity, With<SidebarContent>>,
    existing_buttons: Query<Entity, With<BuildingButton>>,
    registry: Res<BuildingRegistry>,
) {
    if tab_query.is_empty() {
        return;
    }

    let active_tab_type = get_active_tab_type(&all_tabs_query);

    if let Ok(content_entity) = content_query.get_single() {
        update_building_buttons_for_active_tab(
            &mut commands,
            active_tab_type,
            content_entity,
            &registry,
            existing_buttons,
        );
    }
}

pub fn setup_sidebar(mut commands: Commands, registry: Res<BuildingRegistry>) {
    let sidebar = Sidebar::new();
    sidebar.spawn(&mut commands, &registry);
}

pub struct SidebarPlugin;

impl Plugin for SidebarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_sidebar).add_systems(
            Update,
            (
                handle_sidebar_hotkeys.in_set(UISystemSet::InputDetection),
                (
                    handle_sidebar_interactions,
                    update_building_buttons_on_tab_change,
                )
                    .in_set(UISystemSet::EntityManagement),
            ),
        );
    }
}
