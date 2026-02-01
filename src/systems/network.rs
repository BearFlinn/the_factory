use crate::{
    grid::{Layer, Position},
    structures::{
        Building, ConstructionSite, Hub, MultiCellBuilding, NetWorkComponent, BUILDING_LAYER,
    },
};
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

#[derive(Message)]
pub struct NetworkChangedEvent;

#[derive(Component)]
#[allow(dead_code)] // TODO: Figure out if this is needed
pub struct NetworkConnection {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

#[derive(Resource, Default)]
pub struct NetworkConnectivity {
    core_network_cells: HashSet<(i32, i32)>,
    connected_cells: HashSet<(i32, i32)>,
}

impl NetworkConnectivity {
    pub fn is_cell_connected(&self, x: i32, y: i32) -> bool {
        self.connected_cells.contains(&(x, y))
    }

    pub fn is_core_network_cell(&self, x: i32, y: i32) -> bool {
        self.core_network_cells.contains(&(x, y))
    }

    pub fn is_adjacent_to_connected_network(&self, x: i32, y: i32) -> bool {
        let adjacent_positions = [(x, y + 1), (x, y - 1), (x - 1, y), (x + 1, y)];
        adjacent_positions
            .iter()
            .any(|pos| self.connected_cells.contains(pos))
    }

    pub fn is_adjacent_to_core_network(&self, x: i32, y: i32) -> bool {
        let adjacent_positions = [(x, y + 1), (x, y - 1), (x - 1, y), (x + 1, y)];
        adjacent_positions
            .iter()
            .any(|pos| self.core_network_cells.contains(pos))
    }

    /// Adds a cell to the connected network. Only available in test builds.
    #[cfg(test)]
    pub fn add_connected_cell(&mut self, x: i32, y: i32) {
        self.connected_cells.insert((x, y));
    }

    /// Adds a cell to the core network. Only available in test builds.
    #[cfg(test)]
    pub fn add_core_network_cell(&mut self, x: i32, y: i32) {
        self.core_network_cells.insert((x, y));
    }
}

#[must_use]
pub fn calculate_network_connectivity(
    building_layers: &Query<
        (&Position, &Layer, Option<&NetWorkComponent>),
        Or<(With<Building>, With<ConstructionSite>)>,
    >,
    hub: &Query<(&MultiCellBuilding, &Hub)>,
) -> (HashSet<(i32, i32)>, HashSet<(i32, i32)>) {
    let mut core_network_cells = HashSet::new();
    let mut queue = VecDeque::new();

    for (multi_cell, _) in hub.iter() {
        let half_width = multi_cell.width / 2;
        let half_height = multi_cell.height / 2;

        for dy in -half_width..=half_width {
            for dx in -half_width..=half_height {
                let pos = (multi_cell.center_x + dx, multi_cell.center_y + dy);
                if core_network_cells.insert(pos) {
                    queue.push_back(pos);
                }
            }
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        for (adj_x, adj_y) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
            if core_network_cells.contains(&(adj_x, adj_y)) {
                continue;
            }

            let has_connector = building_layers.iter().any(|(position, layer, building)| {
                layer.0 == BUILDING_LAYER
                    && building == Some(&NetWorkComponent)
                    && position.x == adj_x
                    && position.y == adj_y
            });

            if has_connector {
                core_network_cells.insert((adj_x, adj_y));
                queue.push_back((adj_x, adj_y));
            }
        }
    }

    let mut connected_cells = core_network_cells.clone();
    for (position, layer, _) in building_layers.iter() {
        if layer.0 == BUILDING_LAYER {
            let building_pos = (position.x, position.y);

            if core_network_cells.contains(&building_pos) {
                continue;
            }

            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let adjacent = (building_pos.0 + dx, building_pos.1 + dy);
                if core_network_cells.contains(&adjacent) {
                    connected_cells.insert(building_pos);
                    break;
                }
            }
        }
    }

    (core_network_cells, connected_cells)
}

pub fn update_network_connectivity(
    mut network_connectivity: ResMut<NetworkConnectivity>,
    mut network_events: MessageReader<NetworkChangedEvent>,
    building_layers: Query<
        (&Position, &Layer, Option<&NetWorkComponent>),
        Or<(With<Building>, With<ConstructionSite>)>,
    >,
    hub: Query<(&MultiCellBuilding, &Hub)>,
) {
    let should_update =
        !network_events.is_empty() || network_connectivity.core_network_cells.is_empty();

    if !should_update {
        return;
    }

    network_events.clear();

    let (core_network, extended_network) = calculate_network_connectivity(&building_layers, &hub);
    network_connectivity.core_network_cells = core_network;
    network_connectivity.connected_cells = extended_network;
}

pub fn update_visual_network_connections(
    mut commands: Commands,
    mut network_events: MessageReader<NetworkChangedEvent>,
    network_connectivity: Res<NetworkConnectivity>,
    existing_connections: Query<Entity, With<NetworkConnection>>,
) {
    if network_events.is_empty() {
        return;
    }
    network_events.clear();

    for entity in existing_connections.iter() {
        commands.entity(entity).despawn();
    }

    let extended_positions: Vec<_> = network_connectivity
        .connected_cells
        .iter()
        .copied()
        .collect();

    for i in 0..extended_positions.len() {
        for j in (i + 1)..extended_positions.len() {
            let pos1 = extended_positions[i];
            let pos2 = extended_positions[j];

            let dx = (pos2.0 - pos1.0).abs();
            let dy = (pos2.1 - pos1.1).abs();

            if !((dx == 1 && dy == 0) || (dx == 0 && dy == 1)) {
                continue;
            }

            let pos1_is_core = network_connectivity.is_core_network_cell(pos1.0, pos1.1);
            let pos2_is_core = network_connectivity.is_core_network_cell(pos2.0, pos2.1);

            if pos1_is_core || pos2_is_core {
                spawn_connection_visual(&mut commands, pos1, pos2);
            }
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn spawn_connection_visual(commands: &mut Commands, from: (i32, i32), to: (i32, i32)) {
    let from_world = Vec2::new(from.0 as f32 * 64.0, from.1 as f32 * 64.0);
    let to_world = Vec2::new(to.0 as f32 * 64.0, to.1 as f32 * 64.0);

    let center = (from_world + to_world) / 2.0;
    let direction = to_world - from_world;
    let length = direction.length();
    let angle = direction.y.atan2(direction.x);

    commands.spawn((
        NetworkConnection { from, to },
        Sprite::from_color(Color::srgb(0.8, 0.8, 0.2), Vec2::new(length, 4.0)),
        Transform::from_xyz(center.x, center.y, 0.5).with_rotation(Quat::from_rotation_z(angle)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_connectivity_default_is_empty() {
        let connectivity = NetworkConnectivity::default();
        assert!(!connectivity.is_cell_connected(0, 0));
        assert!(!connectivity.is_core_network_cell(0, 0));
    }

    #[test]
    fn is_cell_connected_returns_true_for_connected_cell() {
        let mut connectivity = NetworkConnectivity::default();
        connectivity.add_connected_cell(5, 10);

        assert!(connectivity.is_cell_connected(5, 10));
    }

    #[test]
    fn is_cell_connected_returns_false_for_unconnected_cell() {
        let mut connectivity = NetworkConnectivity::default();
        connectivity.add_connected_cell(5, 10);

        assert!(!connectivity.is_cell_connected(0, 0));
        assert!(!connectivity.is_cell_connected(5, 11));
        assert!(!connectivity.is_cell_connected(6, 10));
    }

    #[test]
    fn is_core_network_cell_returns_true_for_core_cell() {
        let mut connectivity = NetworkConnectivity::default();
        connectivity.add_core_network_cell(3, 7);

        assert!(connectivity.is_core_network_cell(3, 7));
    }

    #[test]
    fn is_core_network_cell_returns_false_for_non_core_cell() {
        let mut connectivity = NetworkConnectivity::default();
        connectivity.add_core_network_cell(3, 7);

        assert!(!connectivity.is_core_network_cell(0, 0));
        assert!(!connectivity.is_core_network_cell(3, 8));
        assert!(!connectivity.is_core_network_cell(4, 7));
    }

    #[test]
    fn is_adjacent_to_connected_network_with_cell_to_north() {
        let mut connectivity = NetworkConnectivity::default();
        // Cell at (5, 6) - north of (5, 5) since y+1
        connectivity.add_connected_cell(5, 6);

        assert!(connectivity.is_adjacent_to_connected_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_connected_network_with_cell_to_south() {
        let mut connectivity = NetworkConnectivity::default();
        // Cell at (5, 4) - south of (5, 5) since y-1
        connectivity.add_connected_cell(5, 4);

        assert!(connectivity.is_adjacent_to_connected_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_connected_network_with_cell_to_east() {
        let mut connectivity = NetworkConnectivity::default();
        // Cell at (6, 5) - east of (5, 5) since x+1
        connectivity.add_connected_cell(6, 5);

        assert!(connectivity.is_adjacent_to_connected_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_connected_network_with_cell_to_west() {
        let mut connectivity = NetworkConnectivity::default();
        // Cell at (4, 5) - west of (5, 5) since x-1
        connectivity.add_connected_cell(4, 5);

        assert!(connectivity.is_adjacent_to_connected_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_connected_network_returns_false_for_no_adjacent() {
        let mut connectivity = NetworkConnectivity::default();
        // Cell at diagonal position (6, 6) - not adjacent to (5, 5)
        connectivity.add_connected_cell(6, 6);

        assert!(!connectivity.is_adjacent_to_connected_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_core_network_with_cell_to_north() {
        let mut connectivity = NetworkConnectivity::default();
        // Core cell at (5, 6) - north of (5, 5)
        connectivity.add_core_network_cell(5, 6);

        assert!(connectivity.is_adjacent_to_core_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_core_network_with_cell_to_south() {
        let mut connectivity = NetworkConnectivity::default();
        // Core cell at (5, 4) - south of (5, 5)
        connectivity.add_core_network_cell(5, 4);

        assert!(connectivity.is_adjacent_to_core_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_core_network_with_cell_to_east() {
        let mut connectivity = NetworkConnectivity::default();
        // Core cell at (6, 5) - east of (5, 5)
        connectivity.add_core_network_cell(6, 5);

        assert!(connectivity.is_adjacent_to_core_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_core_network_with_cell_to_west() {
        let mut connectivity = NetworkConnectivity::default();
        // Core cell at (4, 5) - west of (5, 5)
        connectivity.add_core_network_cell(4, 5);

        assert!(connectivity.is_adjacent_to_core_network(5, 5));
    }

    #[test]
    fn is_adjacent_to_core_network_returns_false_for_no_adjacent() {
        let mut connectivity = NetworkConnectivity::default();
        // Core cell at diagonal position (6, 6) - not adjacent to (5, 5)
        connectivity.add_core_network_cell(6, 6);

        assert!(!connectivity.is_adjacent_to_core_network(5, 5));
    }

    #[test]
    fn connected_cells_and_core_cells_are_independent() {
        let mut connectivity = NetworkConnectivity::default();
        connectivity.add_connected_cell(1, 1);
        connectivity.add_core_network_cell(2, 2);

        // Cell (1, 1) is connected but not core
        assert!(connectivity.is_cell_connected(1, 1));
        assert!(!connectivity.is_core_network_cell(1, 1));

        // Cell (2, 2) is core but may or may not be connected (depends on setup)
        assert!(connectivity.is_core_network_cell(2, 2));
        assert!(!connectivity.is_cell_connected(2, 2));
    }
}
