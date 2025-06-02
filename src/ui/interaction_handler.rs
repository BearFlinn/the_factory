use bevy::prelude::*;

#[derive(Clone)]
pub enum SelectionBehavior {
    Toggle,                  // Click toggles selection on/off
    Exclusive(String),       // Click selects this, deselects others in the group
    Custom(String),          // Custom selection logic (could be extended later)
}

#[derive(Component)]
pub struct Selectable {
    pub is_selected: bool,
    pub selection_behavior: SelectionBehavior,
    pub selection_group: Option<String>, // Elements in the same group deselect each other
}

impl Selectable {
    pub fn new() -> Self {
        Self { 
            is_selected: false,
            selection_behavior: SelectionBehavior::Toggle,
            selection_group: None,
        }
    }
    
    pub fn with_group(mut self, group: String) -> Self {
        self.selection_group = Some(group);
        self
    }
    
    pub fn with_behavior(mut self, behavior: SelectionBehavior) -> Self {
        self.selection_behavior = behavior;
        self
    }
}

#[derive(Clone)]
pub struct DynamicStyles {
    pub background_color: Option<BackgroundColor>,
    pub border_color: Option<BorderColor>,
    // Add more style properties as needed
}

impl DynamicStyles {
    pub fn new() -> Self {
        Self {
            background_color: None,
            border_color: None,
        }
    }
    
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = Some(BackgroundColor(color));
        self
    }
    
    pub fn with_border(mut self, color: Color) -> Self {
        self.border_color = Some(BorderColor(color));
        self
    }
}

#[derive(Component, Clone)]
pub struct InteractiveUI {
    pub default_styles: DynamicStyles,
    pub on_hover: Option<DynamicStyles>,
    pub on_click: Option<DynamicStyles>,
    pub on_selected: Option<DynamicStyles>,
}

impl InteractiveUI {
    pub fn new() -> Self {
        Self {
            default_styles: DynamicStyles::new(),
            on_hover: None,
            on_click: None,
            on_selected: None,
        }
    }
    
    pub fn default(mut self, styles: DynamicStyles) -> Self {
        self.default_styles = styles;
        self
    }
    
    pub fn on_hover(mut self, styles: DynamicStyles) -> Self {
        self.on_hover = Some(styles);
        self
    }
    
    pub fn on_click(mut self, styles: DynamicStyles) -> Self {
        self.on_click = Some(styles);
        self
    }
    
    pub fn selected(mut self, styles: DynamicStyles) -> Self {
        self.on_selected = Some(styles);
        self
    }
}

fn apply_dynamic_styles(
    commands: &mut Commands,
    entity: Entity,
    styles: &DynamicStyles,
) {
    if let Some(bg_color) = &styles.background_color {
        commands.entity(entity).insert(bg_color.clone());
    }
    if let Some(border_color) = &styles.border_color {
        commands.entity(entity).insert(border_color.clone());
    }
}

pub fn handle_interactive_ui(
    mut commands: Commands,
    mut query_set: ParamSet<(
        Query<(Entity, &Interaction, &mut Selectable, &InteractiveUI), Changed<Interaction>>,
        Query<(Entity, &mut Selectable, &InteractiveUI)>,
    )>,
) {
    // First, collect entities that were interacted with and their selection changes
    let mut entities_to_process = Vec::new();
    
    // Process interactions and collect what needs to be done
    for (entity, interaction, mut selectable, interactive_ui) in query_set.p0().iter_mut() {
        if *interaction == Interaction::Pressed {
            match &selectable.selection_behavior {
                SelectionBehavior::Toggle => {
                    selectable.is_selected = !selectable.is_selected;
                }
                SelectionBehavior::Exclusive(group) => {
                    // Store information about what needs to be deselected
                    entities_to_process.push((entity, group.clone(), interactive_ui.clone()));
                    selectable.is_selected = true;
                }
                SelectionBehavior::Custom(behavior_type) => {
                    match behavior_type.as_str() {
                        "building_button" => {
                            selectable.is_selected = true;
                        }
                        _ => {
                            selectable.is_selected = !selectable.is_selected;
                        }
                    }
                }
            }
        }
        
        // Apply visual styles for this entity
        let styles_to_apply = determine_styles(interaction, &selectable, interactive_ui);
        apply_dynamic_styles(&mut commands, entity, styles_to_apply);
    }
    
    // Now handle exclusive deselection using the second query
    for (selected_entity, group, _) in entities_to_process {
        for (other_entity, mut other_selectable, other_ui) in query_set.p1().iter_mut() {
            if other_selectable.selection_group.as_ref() == Some(&group) && other_entity != selected_entity {
                if other_selectable.is_selected {
                    other_selectable.is_selected = false;
                    apply_dynamic_styles(&mut commands, other_entity, &other_ui.default_styles);
                }
            }
        }
    }
}

// Also handle visual updates when selection changes outside of interactions
pub fn update_selection_visuals(
    mut commands: Commands,
    changed_selectables: Query<(Entity, &Selectable, &InteractiveUI, &Interaction), Changed<Selectable>>,
) {
    for (entity, selectable, interactive_ui, interaction) in &changed_selectables {
        let styles_to_apply = determine_styles(interaction, selectable, interactive_ui);
        apply_dynamic_styles(&mut commands, entity, styles_to_apply);
    }
}

fn determine_styles<'a>(
    interaction: &Interaction,
    selectable: &Selectable,
    interactive_ui: &'a InteractiveUI,
) -> &'a DynamicStyles {
    match interaction {
        Interaction::Pressed => {
            if selectable.is_selected && interactive_ui.on_selected.is_some() {
                interactive_ui.on_selected.as_ref().unwrap()
            } else if let Some(click_styles) = &interactive_ui.on_click {
                click_styles
            } else {
                &interactive_ui.default_styles
            }
        }
        Interaction::Hovered => {
            if selectable.is_selected && interactive_ui.on_selected.is_some() {
                interactive_ui.on_selected.as_ref().unwrap()
            } else if let Some(hover_styles) = &interactive_ui.on_hover {
                hover_styles
            } else {
                &interactive_ui.default_styles
            }
        }
        Interaction::None => {
            if selectable.is_selected && interactive_ui.on_selected.is_some() {
                interactive_ui.on_selected.as_ref().unwrap()
            } else {
                &interactive_ui.default_styles
            }
        }
    }
}

pub fn handle_escape_clear_selection(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selectables: Query<&mut Selectable>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        for mut selectable in &mut selectables {
            if selectable.is_selected {
                selectable.is_selected = false;
            }
        }
    }
}

pub struct InteractionHandlerPlugin;

impl Plugin for InteractionHandlerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_escape_clear_selection,
            handle_interactive_ui,
            update_selection_visuals,
        ));
    }
}