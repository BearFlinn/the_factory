use bevy::prelude::*;

use crate::{
    systems::GameScore,
    ui::style::{HEADER_COLOR, PANEL_BG},
};

#[derive(Component)]
pub struct ScoreText;

pub fn setup_score_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                bottom: Val::Px(20.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Score: 0"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(HEADER_COLOR),
                ScoreText,
            ));
        });
}

pub fn update_score_text(score: Res<GameScore>, mut text_query: Query<&mut Text, With<ScoreText>>) {
    if score.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            **text = format!("Score: {}", score.total_score);
        }
    }
}

pub struct ScoreDisplayPlugin;

impl Plugin for ScoreDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_score_ui)
            .add_systems(Update, update_score_text);
    }
}
