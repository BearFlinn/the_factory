pub mod placement;
pub mod workflow_create;

use bevy::prelude::*;

use crate::ui::{UISystemSet, UiMode};

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                placement::update_placement_ghost.run_if(in_state(UiMode::Place)),
                placement::display_placement_error,
                placement::cleanup_placement_errors,
            )
                .in_set(UISystemSet::VisualUpdates),
        );
    }
}
