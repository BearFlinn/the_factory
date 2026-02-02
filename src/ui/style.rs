use bevy::{picking::hover::Hovered, prelude::*, ui::Checked, ui::Pressed};

use crate::ui::UISystemSet;

pub const PANEL_BG: Color = Color::srgba(0.1, 0.1, 0.15, 0.9);
pub const CARD_BG: Color = Color::srgba(0.15, 0.15, 0.2, 0.8);
pub const POPUP_BG: Color = Color::srgba(0.1, 0.1, 0.15, 0.95);
pub const PANEL_BORDER: Color = Color::srgb(0.3, 0.4, 0.6);

pub const HEADER_COLOR: Color = Color::srgb(0.85, 0.85, 0.95);
pub const TEXT_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
pub const DIM_TEXT: Color = Color::srgb(0.5, 0.5, 0.5);

pub const BUTTON_BG: Color = Color::srgb(0.2, 0.2, 0.3);
pub const BUTTON_HOVER: Color = Color::srgb(0.3, 0.3, 0.45);

pub const CONFIRM_BG: Color = Color::srgb(0.15, 0.35, 0.15);
pub const CONFIRM_HOVER: Color = Color::srgb(0.2, 0.5, 0.2);

pub const CANCEL_BG: Color = Color::srgb(0.35, 0.15, 0.15);
pub const CANCEL_HOVER: Color = Color::srgb(0.5, 0.2, 0.2);

pub const SELECTED_BG: Color = Color::srgb(0.15, 0.25, 0.35);
pub const SELECTED_BORDER: Color = Color::srgb(0.3, 0.5, 0.7);

#[derive(Component, Clone)]
pub struct ButtonStyle {
    pub default_bg: Color,
    pub hovered_bg: Color,
    pub default_border: Option<Color>,
    pub hovered_border: Option<Color>,
    pub selected_bg: Option<Color>,
    pub selected_border: Option<Color>,
}

impl ButtonStyle {
    #[must_use]
    pub fn default_button() -> Self {
        Self {
            default_bg: BUTTON_BG,
            hovered_bg: BUTTON_HOVER,
            default_border: None,
            hovered_border: None,
            selected_bg: None,
            selected_border: None,
        }
    }

    #[must_use]
    pub fn confirm() -> Self {
        Self {
            default_bg: CONFIRM_BG,
            hovered_bg: CONFIRM_HOVER,
            default_border: None,
            hovered_border: None,
            selected_bg: None,
            selected_border: None,
        }
    }

    #[must_use]
    pub fn cancel() -> Self {
        Self {
            default_bg: CANCEL_BG,
            hovered_bg: CANCEL_HOVER,
            default_border: None,
            hovered_border: None,
            selected_bg: None,
            selected_border: None,
        }
    }

    #[must_use]
    pub fn close() -> Self {
        Self {
            default_bg: CANCEL_BG,
            hovered_bg: CANCEL_HOVER,
            default_border: None,
            hovered_border: None,
            selected_bg: None,
            selected_border: None,
        }
    }

    #[must_use]
    pub fn tab() -> Self {
        Self {
            default_bg: BUTTON_BG,
            hovered_bg: BUTTON_HOVER,
            default_border: Some(PANEL_BORDER),
            hovered_border: Some(PANEL_BORDER),
            selected_bg: Some(SELECTED_BG),
            selected_border: Some(SELECTED_BORDER),
        }
    }

    #[must_use]
    pub fn building_button() -> Self {
        Self {
            default_bg: BUTTON_BG,
            hovered_bg: BUTTON_HOVER,
            default_border: Some(PANEL_BORDER),
            hovered_border: Some(PANEL_BORDER),
            selected_bg: Some(SELECTED_BG),
            selected_border: Some(SELECTED_BORDER),
        }
    }
}

pub fn apply_button_styles(
    mut buttons: Query<
        (
            &ButtonStyle,
            &mut BackgroundColor,
            Option<&mut BorderColor>,
            Has<Pressed>,
            &Hovered,
            Has<Checked>,
        ),
        Or<(Changed<Pressed>, Changed<Hovered>, Added<Checked>)>,
    >,
) {
    for (style, mut bg, border, pressed, hovered, checked) in &mut buttons {
        apply_style(style, &mut bg, border, pressed, hovered.0, checked);
    }
}

pub fn apply_button_styles_on_uncheck(
    mut buttons: Query<
        (
            &ButtonStyle,
            &mut BackgroundColor,
            Option<&mut BorderColor>,
            Has<Pressed>,
            &Hovered,
            Has<Checked>,
        ),
        With<ButtonStyle>,
    >,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_pressed: RemovedComponents<Pressed>,
) {
    for entity in removed_checked.read().chain(removed_pressed.read()) {
        if let Ok((style, mut bg, border, pressed, hovered, checked)) = buttons.get_mut(entity) {
            apply_style(style, &mut bg, border, pressed, hovered.0, checked);
        }
    }
}

fn apply_style(
    style: &ButtonStyle,
    bg: &mut BackgroundColor,
    border: Option<Mut<BorderColor>>,
    pressed: bool,
    hovered: bool,
    checked: bool,
) {
    if checked {
        if let Some(sel_bg) = style.selected_bg {
            *bg = BackgroundColor(sel_bg);
        } else {
            *bg = BackgroundColor(style.default_bg);
        }
        if let (Some(mut bc), Some(sel_border)) = (border, style.selected_border) {
            bc.set_all(sel_border);
        }
    } else if pressed || hovered {
        *bg = BackgroundColor(style.hovered_bg);
        if let (Some(mut bc), Some(hov_border)) = (border, style.hovered_border) {
            bc.set_all(hov_border);
        }
    } else {
        *bg = BackgroundColor(style.default_bg);
        if let Some(mut bc) = border {
            if let Some(def_border) = style.default_border {
                bc.set_all(def_border);
            }
        }
    }
}

pub struct StylePlugin;

impl Plugin for StylePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (apply_button_styles, apply_button_styles_on_uncheck)
                .in_set(UISystemSet::VisualUpdates),
        );
    }
}
