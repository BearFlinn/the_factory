// Library target exists for integration tests only — suppress library-API lints
// that don't apply to a game crate.
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::implicit_hasher
)]

pub mod camera;
pub mod constants;
pub mod grid;
pub mod materials;
pub mod resources;
pub mod structures;
pub mod systems;
pub mod ui;
pub mod workers;

#[cfg(debug_assertions)]
pub mod invariants;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameplaySet {
    GridUpdate,
    ResourceSpawning,
    SystemsUpdate,
    DomainOperations,
    UIUpdate,
}

pub fn configure_system_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            GameplaySet::GridUpdate,
            GameplaySet::ResourceSpawning,
            GameplaySet::SystemsUpdate,
            GameplaySet::DomainOperations,
            GameplaySet::UIUpdate,
        )
            .chain(),
    );
}
