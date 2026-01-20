use bevy::prelude::*;

use crate::systems::GameScore;

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
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Score: 0"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.2)),
                ScoreText,
            ));
        });
}

pub fn update_score_text(score: Res<GameScore>, mut text_query: Query<&mut Text, With<ScoreText>>) {
    if score.is_changed() {
        if let Ok(mut text) = text_query.get_single_mut() {
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
