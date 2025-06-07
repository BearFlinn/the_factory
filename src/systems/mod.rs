// systems/mod.rs
pub mod power;
pub mod compute;
pub mod network;
pub mod operational;
pub mod display;

pub use power::*;
pub use compute::*;
pub use network::*;
pub use operational::*;
pub use display::*;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SystemsSet {
    Infrastructure,  // power, compute, network
    Operational,     // operational status calculation
    Display,         // visual indicators
}

pub struct SystemsPlugin;

impl Plugin for SystemsPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PowerGrid::default())
            .insert_resource(ComputeGrid::default())
            .insert_resource(NetworkConnectivity::default())
            .add_event::<NetworkChangedEvent>()
            .configure_sets(Update, (
                SystemsSet::Infrastructure,
                SystemsSet::Operational,
                SystemsSet::Display,
            ).chain().in_set(crate::GameplaySet::SystemsUpdate))
            .add_systems(Update, (
                (update_power_grid, update_compute, update_network_connectivity)
                    .in_set(SystemsSet::Infrastructure),
                
                (update_operational_status
                ).in_set(SystemsSet::Operational),
                
                (update_inventory_display,
                update_operational_indicators,
                update_visual_network_connections,
                update_placement_ghost,
                display_placement_error,
                cleanup_placement_errors
                ).in_set(SystemsSet::Display),
            ));
    }
}