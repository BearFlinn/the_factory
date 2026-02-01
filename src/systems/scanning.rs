use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    grid::{ExpandGridCellsEvent, Grid, Position},
    structures::Building,
};

#[derive(Component)]
pub struct Scanner {
    pub base_scan_interval: f32,
    pub scan_timer: Timer,
    pub position: Position,
    pub last_scan_angle: f32,
    pub current_target_distance: i32,
    pub sector_size: i32, // Grid spacing AND reveal size (e.g., 5 = 5x5 sectors)
}

impl Scanner {
    pub fn new(base_scan_interval: f32, position: Position) -> Self {
        Self {
            base_scan_interval,
            scan_timer: Timer::from_seconds(base_scan_interval, TimerMode::Once),
            position,
            last_scan_angle: 0.0,
            current_target_distance: 1,
            sector_size: 5,
        }
    }

    /// Calculate scan time based on distance (linear scaling with reasonable cap)
    #[allow(clippy::cast_precision_loss)]
    fn calculate_scan_time(&self, distance: i32) -> f32 {
        let max_time = self.base_scan_interval * 10.0; // Cap at 10x base time
        (self.base_scan_interval * distance as f32).min(max_time)
    }

    /// Calculate angle from scanner to point, with North=0, increasing clockwise
    #[allow(clippy::cast_precision_loss)]
    fn calculate_angle(&self, x: i32, y: i32) -> f32 {
        let dx = (x - self.position.x) as f32;
        let dy = (y - self.position.y) as f32;

        // atan2(y, x) gives angle from positive X axis
        // We want angle from positive Y axis (North), so we use atan2(x, y)
        let angle = dx.atan2(dy);

        // Normalize to [0, 2π] range
        if angle < 0.0 {
            angle + 2.0 * std::f32::consts::PI
        } else {
            angle
        }
    }

    /// Convert a world coordinate to its sector center
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn to_sector_center(&self, x: i32, y: i32) -> (i32, i32) {
        let sector_x = ((x as f32 / self.sector_size as f32).round() as i32) * self.sector_size;
        let sector_y = ((y as f32 / self.sector_size as f32).round() as i32) * self.sector_size;
        (sector_x, sector_y)
    }

    /// Check if a sector is fully explored (all tiles revealed)
    fn is_sector_fully_explored(&self, sector_x: i32, sector_y: i32, grid: &Grid) -> bool {
        let half = self.sector_size / 2;
        for dy in -half..=half {
            for dx in -half..=half {
                if !grid
                    .valid_coordinates
                    .contains(&(sector_x + dx, sector_y + dy))
                {
                    return false;
                }
            }
        }
        true
    }

    fn find_exploration_targets(&self, grid: &Grid) -> Vec<(i32, i32, i32, f32)> {
        // Find sectors containing unexplored tiles adjacent to explored tiles
        let mut candidate_sectors: HashSet<(i32, i32)> = HashSet::new();

        for &(x, y) in &grid.valid_coordinates {
            let neighbors = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
            for (nx, ny) in neighbors {
                if !grid.valid_coordinates.contains(&(nx, ny)) {
                    // Found an unexplored tile adjacent to explored area
                    // Add its sector as a candidate
                    candidate_sectors.insert(self.to_sector_center(nx, ny));
                }
            }
        }

        // Filter out fully explored sectors
        let candidate_sectors: HashSet<_> = candidate_sectors
            .into_iter()
            .filter(|&(sx, sy)| !self.is_sector_fully_explored(sx, sy, grid))
            .collect();

        // Convert to target format with distance and angle
        let mut targets: Vec<(i32, i32, i32, f32)> = candidate_sectors
            .into_iter()
            .map(|(x, y)| {
                let distance = (x - self.position.x).abs().max((y - self.position.y).abs());
                let angle = self.calculate_angle(x, y);
                (x, y, distance, angle)
            })
            .collect();

        // Sort by distance first, then clockwise by angle
        targets.sort_by(|a, b| {
            let distance_cmp = a.2.cmp(&b.2);
            if distance_cmp != std::cmp::Ordering::Equal {
                return distance_cmp;
            }

            let angle_diff_a = self.calculate_angle_diff(a.3);
            let angle_diff_b = self.calculate_angle_diff(b.3);
            angle_diff_a
                .partial_cmp(&angle_diff_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        targets
    }

    fn calculate_angle_diff(&self, target_angle: f32) -> f32 {
        let mut diff = target_angle - self.last_scan_angle;

        // Normalize to [0, 2π] to ensure clockwise preference
        while diff < 0.0 {
            diff += 2.0 * std::f32::consts::PI;
        }
        while diff >= 2.0 * std::f32::consts::PI {
            diff -= 2.0 * std::f32::consts::PI;
        }

        diff
    }

    pub fn find_next_cluster(&mut self, grid: &Grid) -> Option<(Vec<(i32, i32)>, i32)> {
        let targets = self.find_exploration_targets(grid);

        if targets.is_empty() {
            return None;
        }

        let (target_x, target_y, distance, angle) = targets[0];

        self.last_scan_angle = angle;

        let mut cluster = Vec::new();
        let half = self.sector_size / 2;
        for dy in -half..=half {
            for dx in -half..=half {
                cluster.push((target_x + dx, target_y + dy));
            }
        }

        Some((cluster, distance))
    }

    pub fn reset_timer_for_distance(&mut self, distance: i32) {
        self.current_target_distance = distance;
        let scan_time = self.calculate_scan_time(distance);
        self.scan_timer = Timer::from_seconds(scan_time, TimerMode::Once);
    }
}

pub fn handle_progressive_scanning(
    mut scanners: Query<&mut Scanner, With<Building>>,
    mut expand_events: MessageWriter<ExpandGridCellsEvent>,
    grid: Res<Grid>,
    time: Res<Time>,
) {
    for mut scanner in &mut scanners {
        scanner.scan_timer.tick(time.delta());

        if scanner.scan_timer.just_finished() {
            if let Some((cluster, target_distance)) = scanner.find_next_cluster(&grid) {
                expand_events.write(ExpandGridCellsEvent {
                    coordinates: cluster,
                });

                scanner.reset_timer_for_distance(target_distance);

                println!(
                    "Scanner at ({}, {}) revealed cluster at distance {} (scan time: {:.1}s)",
                    scanner.position.x,
                    scanner.position.y,
                    target_distance,
                    scanner.calculate_scan_time(target_distance)
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn create_scanner_at_origin(base_interval: f32) -> Scanner {
        Scanner::new(base_interval, Position { x: 0, y: 0 })
    }

    #[test]
    fn calculate_scan_time_base_for_adjacent() {
        let scanner = create_scanner_at_origin(1.0);

        // Distance 1 should return base scan time
        let time = scanner.calculate_scan_time(1);
        assert!(
            (time - 1.0).abs() < f32::EPSILON,
            "Expected 1.0, got {time}"
        );
    }

    #[test]
    fn calculate_scan_time_scales_with_distance() {
        let scanner = create_scanner_at_origin(1.0);

        // Distance 5 should return 5x base time
        let time = scanner.calculate_scan_time(5);
        assert!(
            (time - 5.0).abs() < f32::EPSILON,
            "Expected 5.0, got {time}"
        );

        // Distance 3 should return 3x base time
        let time = scanner.calculate_scan_time(3);
        assert!(
            (time - 3.0).abs() < f32::EPSILON,
            "Expected 3.0, got {time}"
        );
    }

    #[test]
    fn calculate_scan_time_caps_at_maximum() {
        let scanner = create_scanner_at_origin(1.0);

        // Maximum time is 10x base (10.0 for base 1.0)
        // Distance 15 would be 15.0, but should cap at 10.0
        let time = scanner.calculate_scan_time(15);
        assert!(
            (time - 10.0).abs() < f32::EPSILON,
            "Expected 10.0 (capped), got {time}"
        );

        // Distance 100 should also cap at 10.0
        let time = scanner.calculate_scan_time(100);
        assert!(
            (time - 10.0).abs() < f32::EPSILON,
            "Expected 10.0 (capped), got {time}"
        );
    }

    #[test]
    fn calculate_scan_time_with_different_base_intervals() {
        let scanner = create_scanner_at_origin(2.5);

        // Distance 1 should return base time (2.5)
        let time = scanner.calculate_scan_time(1);
        assert!(
            (time - 2.5).abs() < f32::EPSILON,
            "Expected 2.5, got {time}"
        );

        // Distance 4 should return 4 * 2.5 = 10.0
        let time = scanner.calculate_scan_time(4);
        assert!(
            (time - 10.0).abs() < f32::EPSILON,
            "Expected 10.0, got {time}"
        );

        // Max for base 2.5 is 25.0 (10x)
        let time = scanner.calculate_scan_time(20);
        assert!(
            (time - 25.0).abs() < f32::EPSILON,
            "Expected 25.0 (capped), got {time}"
        );
    }

    #[test]
    fn calculate_angle_north_is_zero() {
        let scanner = create_scanner_at_origin(1.0);

        // Point directly north (positive y) should be angle 0
        let angle = scanner.calculate_angle(0, 5);
        assert!(angle.abs() < 0.001, "North should be angle 0, got {angle}");
    }

    #[test]
    fn calculate_angle_east_is_quarter_turn() {
        let scanner = create_scanner_at_origin(1.0);

        // Point directly east (positive x) should be PI/2 (90 degrees)
        let angle = scanner.calculate_angle(5, 0);
        let expected = std::f32::consts::FRAC_PI_2;
        assert!(
            (angle - expected).abs() < 0.001,
            "East should be PI/2 ({expected}), got {angle}"
        );
    }

    #[test]
    fn calculate_angle_south_is_half_turn() {
        let scanner = create_scanner_at_origin(1.0);

        // Point directly south (negative y) should be PI (180 degrees)
        let angle = scanner.calculate_angle(0, -5);
        let expected = std::f32::consts::PI;
        assert!(
            (angle - expected).abs() < 0.001,
            "South should be PI ({expected}), got {angle}"
        );
    }

    #[test]
    fn calculate_angle_west_is_three_quarter_turn() {
        let scanner = create_scanner_at_origin(1.0);

        // Point directly west (negative x) should be 3*PI/2 (270 degrees)
        let angle = scanner.calculate_angle(-5, 0);
        let expected = 3.0 * std::f32::consts::FRAC_PI_2;
        assert!(
            (angle - expected).abs() < 0.001,
            "West should be 3*PI/2 ({expected}), got {angle}"
        );
    }

    #[test]
    fn calculate_angle_diff_clockwise_progression() {
        let mut scanner = create_scanner_at_origin(1.0);
        scanner.last_scan_angle = 0.0; // Start at north

        // Angle slightly clockwise (east direction) should be small positive diff
        let diff = scanner.calculate_angle_diff(std::f32::consts::FRAC_PI_4);
        assert!(
            (diff - std::f32::consts::FRAC_PI_4).abs() < 0.001,
            "Clockwise quarter turn should have diff PI/4, got {diff}"
        );
    }

    #[test]
    fn calculate_angle_diff_wraps_correctly() {
        let mut scanner = create_scanner_at_origin(1.0);
        scanner.last_scan_angle = 3.0 * std::f32::consts::FRAC_PI_2; // At west (270 degrees)

        // Target at north (0 degrees) should be a small clockwise step
        let diff = scanner.calculate_angle_diff(0.0);
        let expected = std::f32::consts::FRAC_PI_2; // 90 degrees to complete the circle
        assert!(
            (diff - expected).abs() < 0.001,
            "Expected wrap-around diff of PI/2 ({expected}), got {diff}"
        );
    }

    #[test]
    fn scanner_new_initializes_correctly() {
        let pos = Position { x: 10, y: 20 };
        let scanner = Scanner::new(3.0, pos);

        assert!((scanner.base_scan_interval - 3.0).abs() < f32::EPSILON);
        assert_eq!(scanner.position.x, 10);
        assert_eq!(scanner.position.y, 20);
        assert!(
            scanner.last_scan_angle.abs() < f32::EPSILON,
            "Should start at angle 0 (north)"
        );
        assert_eq!(
            scanner.current_target_distance, 1,
            "Should start at distance 1"
        );
    }

    #[test]
    fn reset_timer_for_distance_updates_state() {
        let mut scanner = create_scanner_at_origin(2.0);

        scanner.reset_timer_for_distance(5);

        assert_eq!(scanner.current_target_distance, 5);
        // Timer should be set to calculated scan time (5 * 2.0 = 10.0)
        assert!(
            (scanner.scan_timer.duration().as_secs_f32() - 10.0).abs() < 0.01,
            "Timer duration should be 10.0"
        );
    }

    fn create_grid_with_coordinates(coords: &[(i32, i32)]) -> Grid {
        let mut grid = Grid::new(1.0);
        for &(x, y) in coords {
            grid.add_coordinate(x, y);
        }
        grid
    }

    #[test]
    fn to_sector_center_rounds_to_nearest_sector() {
        let scanner = create_scanner_at_origin(1.0);

        // With sector_size=5, coordinates round to nearest multiple of 5
        assert_eq!(scanner.to_sector_center(0, 0), (0, 0));
        assert_eq!(scanner.to_sector_center(2, 2), (0, 0));
        assert_eq!(scanner.to_sector_center(3, 3), (5, 5));
        assert_eq!(scanner.to_sector_center(-2, -2), (0, 0));
        assert_eq!(scanner.to_sector_center(-3, -3), (-5, -5));
        assert_eq!(scanner.to_sector_center(7, 8), (5, 10));
    }

    #[test]
    fn find_exploration_targets_returns_sector_centers() {
        let scanner = create_scanner_at_origin(1.0);
        // Explore the origin sector
        let grid = create_grid_with_coordinates(&[(0, 0)]);

        let targets = scanner.find_exploration_targets(&grid);

        // All targets should be on the sector grid (multiples of 5)
        for (x, y, _, _) in &targets {
            assert_eq!(
                x % scanner.sector_size,
                0,
                "Target x={x} should be multiple of sector_size"
            );
            assert_eq!(
                y % scanner.sector_size,
                0,
                "Target y={y} should be multiple of sector_size"
            );
        }
    }

    #[test]
    fn find_exploration_targets_finds_sectors_with_unexplored_tiles() {
        let scanner = create_scanner_at_origin(1.0);
        // Explore just one tile - its neighbors are unexplored
        let grid = create_grid_with_coordinates(&[(0, 0)]);

        let targets = scanner.find_exploration_targets(&grid);

        // With just (0,0) explored, unexplored neighbors (1,0), (-1,0), etc.
        // all belong to sector (0,0), so we find that sector
        assert!(
            !targets.is_empty(),
            "Should find at least one sector with unexplored tiles"
        );

        // The sector containing unexplored neighbors should be (0,0)
        let target_coords: std::collections::HashSet<(i32, i32)> =
            targets.iter().map(|(x, y, _, _)| (*x, *y)).collect();
        assert!(
            target_coords.contains(&(0, 0)),
            "Should find sector (0, 0) which has unexplored tiles"
        );
    }

    #[test]
    fn find_exploration_targets_finds_adjacent_sectors_when_current_full() {
        let scanner = create_scanner_at_origin(1.0);
        // Fully explore the origin sector (5x5 area centered at 0,0)
        let mut coords = Vec::new();
        for y in -2..=2 {
            for x in -2..=2 {
                coords.push((x, y));
            }
        }
        let grid = create_grid_with_coordinates(&coords);

        let targets = scanner.find_exploration_targets(&grid);

        // Now adjacent sectors should be found
        let target_coords: std::collections::HashSet<(i32, i32)> =
            targets.iter().map(|(x, y, _, _)| (*x, *y)).collect();

        // Should find sectors at distance 5 (adjacent to the filled sector)
        assert!(
            target_coords.contains(&(5, 0)) || target_coords.contains(&(0, 5)),
            "Should find adjacent sectors when current sector is full"
        );
    }

    #[test]
    fn find_exploration_targets_sorts_by_distance_then_angle() {
        let mut scanner = create_scanner_at_origin(1.0);
        scanner.last_scan_angle = 0.0;

        let grid = create_grid_with_coordinates(&[(0, 0)]);
        let targets = scanner.find_exploration_targets(&grid);

        // Verify sorting: distance first, then angle
        for i in 1..targets.len() {
            let (_, _, dist_prev, angle_prev) = targets[i - 1];
            let (_, _, dist_curr, angle_curr) = targets[i];

            if dist_prev == dist_curr {
                let angle_diff_prev = scanner.calculate_angle_diff(angle_prev);
                let angle_diff_curr = scanner.calculate_angle_diff(angle_curr);
                assert!(
                    angle_diff_curr >= angle_diff_prev - 0.001,
                    "Within same distance, sectors should be sorted by angle"
                );
            } else {
                assert!(
                    dist_curr >= dist_prev,
                    "Sectors should be sorted by distance first"
                );
            }
        }
    }

    #[test]
    fn find_next_cluster_reveals_sector_size_area() {
        let mut scanner = create_scanner_at_origin(1.0);
        // Default sector_size=5, reveals 5x5=25 tiles
        let grid = create_grid_with_coordinates(&[(0, 0)]);

        let (cluster, _) = scanner.find_next_cluster(&grid).unwrap();

        assert_eq!(
            cluster.len(),
            25,
            "Should reveal 5x5 area with sector_size=5"
        );
    }

    #[test]
    fn find_next_cluster_respects_sector_size() {
        let mut scanner = create_scanner_at_origin(1.0);
        scanner.sector_size = 3; // 3x3 sectors

        let grid = create_grid_with_coordinates(&[(0, 0)]);

        let (cluster, _) = scanner.find_next_cluster(&grid).unwrap();

        // sector_size=3, half=1, range -1..=1 = 3x3 = 9 tiles
        assert_eq!(
            cluster.len(),
            9,
            "Should reveal 3x3 area with sector_size=3"
        );
    }

    #[test]
    fn consecutive_sector_scans_share_edges_not_overlap() {
        let mut scanner = create_scanner_at_origin(1.0);
        let mut grid = create_grid_with_coordinates(&[(0, 0)]);

        // Simulate two consecutive scans
        let (cluster1, _) = scanner.find_next_cluster(&grid).unwrap();
        for (x, y) in &cluster1 {
            grid.add_coordinate(*x, *y);
        }

        let (cluster2, _) = scanner.find_next_cluster(&grid).unwrap();

        // With sector_size=5, sectors are 5 apart and reveal 5x5
        // Adjacent sectors share one edge but interior tiles don't overlap
        let set1: std::collections::HashSet<_> = cluster1.into_iter().collect();
        let set2: std::collections::HashSet<_> = cluster2.into_iter().collect();
        let overlap: Vec<_> = set1.intersection(&set2).collect();

        // Edge sharing means at most 5 tiles overlap (one row/column)
        assert!(
            overlap.len() <= 5,
            "Adjacent sectors should share at most one edge (5 tiles), but found {} overlapping",
            overlap.len()
        );
    }
}
