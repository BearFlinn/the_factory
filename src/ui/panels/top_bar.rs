use bevy::prelude::*;

use crate::{
    systems::{ComputeGrid, GameScore, PowerGrid},
    ui::{
        icons::{spawn_icon, GameIcon, IconAtlas},
        style::{
            COMPUTE_COLOR, DANGER_COLOR, PANEL_BORDER, POWER_COLOR, SCORE_COLOR, TOP_BAR_BG,
            TOP_BAR_HEIGHT, WARNING_COLOR, WORKER_COLOR,
        },
        UISystemSet,
    },
    workers::Worker,
};

#[derive(Component)]
pub struct TopBar;

#[derive(Component)]
pub struct TopBarPowerText;

#[derive(Component)]
pub struct TopBarComputeText;

#[derive(Component)]
pub struct TopBarWorkerText;

#[derive(Component)]
pub struct TopBarScoreText;

fn setup_top_bar(mut commands: Commands, icon_atlas: Res<IconAtlas>) {
    let bar = commands
        .spawn((
            Node {
                width: Val::Percent(80.0),
                height: Val::Px(TOP_BAR_HEIGHT),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Percent(10.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(12.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(TOP_BAR_BG),
            BorderColor::all(PANEL_BORDER),
            TopBar,
        ))
        .id();

    let left_section = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(20.0),
            ..default()
        })
        .id();

    let power_group = spawn_stat_group(
        &mut commands,
        &icon_atlas,
        GameIcon::Power,
        "0/0",
        POWER_COLOR,
        TopBarPowerText,
    );

    let compute_group = spawn_stat_group(
        &mut commands,
        &icon_atlas,
        GameIcon::Compute,
        "0/0",
        COMPUTE_COLOR,
        TopBarComputeText,
    );

    let worker_group = spawn_stat_group(
        &mut commands,
        &icon_atlas,
        GameIcon::Workers,
        "0",
        WORKER_COLOR,
        TopBarWorkerText,
    );

    commands
        .entity(left_section)
        .add_children(&[power_group, compute_group, worker_group]);

    let right_section = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let score_group = spawn_stat_group(
        &mut commands,
        &icon_atlas,
        GameIcon::Score,
        "0",
        SCORE_COLOR,
        TopBarScoreText,
    );

    commands.entity(right_section).add_child(score_group);
    commands
        .entity(bar)
        .add_children(&[left_section, right_section]);
}

fn spawn_stat_group(
    commands: &mut Commands,
    icon_atlas: &IconAtlas,
    icon: GameIcon,
    initial_text: &str,
    color: Color,
    marker: impl Component,
) -> Entity {
    let group = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .id();

    let icon_entity = spawn_icon(commands, icon_atlas, icon, 18.0);

    let text_entity = commands
        .spawn((
            Text::new(initial_text),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(color),
            marker,
        ))
        .id();

    commands
        .entity(group)
        .add_children(&[icon_entity, text_entity]);
    group
}

fn update_power_text(
    power_grid: Res<PowerGrid>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<TopBarPowerText>>,
) {
    if !power_grid.is_changed() {
        return;
    }

    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = format!("{}/{}", power_grid.available, power_grid.capacity);
        color.0 = stat_color(power_grid.available, power_grid.capacity, POWER_COLOR);
    }
}

fn update_compute_text(
    compute_grid: Res<ComputeGrid>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<TopBarComputeText>>,
) {
    if !compute_grid.is_changed() {
        return;
    }

    if let Ok((mut text, mut color)) = text_query.single_mut() {
        **text = format!("{}/{}", compute_grid.available, compute_grid.capacity);
        color.0 = stat_color(compute_grid.available, compute_grid.capacity, COMPUTE_COLOR);
    }
}

fn update_worker_text(
    workers: Query<(), With<Worker>>,
    mut text_query: Query<&mut Text, With<TopBarWorkerText>>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        let count = workers.iter().count();
        **text = format!("{count}");
    }
}

fn update_score_text(
    score: Res<GameScore>,
    mut text_query: Query<&mut Text, With<TopBarScoreText>>,
) {
    if !score.is_changed() {
        return;
    }

    if let Ok(mut text) = text_query.single_mut() {
        **text = format_score(score.total_score);
    }
}

fn stat_color(available: i32, capacity: i32, default_color: Color) -> Color {
    if available <= 0 {
        return DANGER_COLOR;
    }
    if capacity > 0 {
        #[allow(clippy::cast_precision_loss)]
        let ratio = available as f32 / capacity as f32;
        if ratio < 0.25 {
            return WARNING_COLOR;
        }
    }
    default_color
}

fn format_score(score: u64) -> String {
    if score >= 1_000_000 {
        #[allow(clippy::cast_precision_loss)]
        let m = score as f64 / 1_000_000.0;
        format!("{m:.1}M")
    } else if score >= 1_000 {
        let formatted = score.to_string();
        let mut result = String::new();
        for (i, ch) in formatted.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(ch);
        }
        result.chars().rev().collect()
    } else {
        score.to_string()
    }
}

pub struct TopBarPlugin;

impl Plugin for TopBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_top_bar).add_systems(
            Update,
            (
                update_power_text,
                update_compute_text,
                update_worker_text,
                update_score_text,
            )
                .in_set(UISystemSet::VisualUpdates),
        );
    }
}
