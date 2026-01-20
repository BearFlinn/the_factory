#![allow(unused_imports)]

pub mod compute;
pub mod display;
pub mod network;
pub mod operational;
pub mod power;
pub mod scanning;

pub use compute::{update_compute, ComputeGrid};
pub use display::{
    cleanup_placement_errors, display_placement_error, update_inventory_display,
    update_operational_indicators, update_placement_ghost, InventoryDisplay,
    NonOperationalIndicator, PlacementErrorMessage, PlacementGhost,
};
pub use network::{
    calculate_network_connectivity, update_network_connectivity, update_visual_network_connections,
    NetworkChangedEvent, NetworkConnection, NetworkConnectivity,
};
pub use operational::{
    populate_operational_conditions, update_operational_status, Operational, OperationalCondition,
};
pub use power::{update_power_grid, PowerGrid};
pub use scanning::{handle_progressive_scanning, Scanner};

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SystemsSet {
    Infrastructure,
    Operational,
    Display,
}

pub struct SystemsPlugin;

impl Plugin for SystemsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PowerGrid::default())
            .insert_resource(ComputeGrid::default())
            .insert_resource(NetworkConnectivity::default())
            .add_event::<NetworkChangedEvent>()
            .configure_sets(
                Update,
                (
                    SystemsSet::Infrastructure,
                    SystemsSet::Operational,
                    SystemsSet::Display,
                )
                    .chain()
                    .in_set(crate::GameplaySet::SystemsUpdate),
            )
            .add_systems(
                Update,
                (
                    (
                        (
                            update_power_grid,
                            update_compute,
                            update_network_connectivity,
                        ),
                        (handle_progressive_scanning).chain(),
                    )
                        .in_set(SystemsSet::Infrastructure),
                    (populate_operational_conditions, update_operational_status)
                        .chain()
                        .in_set(SystemsSet::Operational),
                    (
                        update_inventory_display,
                        update_operational_indicators,
                        update_visual_network_connections,
                        update_placement_ghost,
                        display_placement_error,
                        cleanup_placement_errors,
                    )
                        .in_set(SystemsSet::Display),
                ),
            );
    }
}
