#![allow(unused_imports)]

pub mod compute;
pub mod display;
pub mod network;
pub mod operational;
pub mod power;
pub mod scanning;

pub use compute::{update_compute, ComputeGrid};
pub use display::{
    update_inventory_display, update_operational_indicators, InventoryDisplay,
    NonOperationalIndicator,
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

#[derive(Resource, Default)]
pub struct GameScore {
    pub total_score: u64,
    pub launches_completed: u32,
}

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
            .init_resource::<GameScore>()
            .add_message::<NetworkChangedEvent>()
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
                    )
                        .in_set(SystemsSet::Display),
                ),
            );
    }
}
