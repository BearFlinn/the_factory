use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    grid::{ExpandGridCellsEvent, Grid, Position},
    structures::Building,
};

#[derive(Component)]
pub struct Scanner {
    pub base_scan_interval: f32, // Base time to scan adjacent tiles
    pub scan_timer: Timer,
    pub position: Position,
    pub last_scan_angle: f32,         // Track progress around perimeter
    pub current_target_distance: i32, // Distance to current scan target
}

impl Scanner {
    pub fn new(base_scan_interval: f32, position: Position) -> Self {
        Self {
            base_scan_interval,
            scan_timer: Timer::from_seconds(base_scan_interval, TimerMode::Once),
            position,
            last_scan_angle: 0.0,       // Start from north
            current_target_distance: 1, // Start with adjacent tiles
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

    fn find_exploration_targets(&self, grid: &Grid) -> Vec<(i32, i32, i32, f32)> {
        // Use HashMap to deduplicate by (x, y) - keeping the entry with minimum distance
        let mut target_map: HashMap<(i32, i32), (i32, f32)> = HashMap::new();

        for &(x, y) in &grid.valid_coordinates {
            let neighbors = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
            for (nx, ny) in neighbors {
                let neighbor_distance = (nx - self.position.x)
                    .abs()
                    .max((ny - self.position.y).abs());

                if !grid.valid_coordinates.contains(&(nx, ny)) {
                    let angle = self.calculate_angle(nx, ny);
                    target_map
                        .entry((nx, ny))
                        .and_modify(|(existing_dist, _)| {
                            if neighbor_distance < *existing_dist {
                                *existing_dist = neighbor_distance;
                            }
                        })
                        .or_insert((neighbor_distance, angle));
                }
            }
        }

        let mut targets: Vec<(i32, i32, i32, f32)> = target_map
            .into_iter()
            .map(|((x, y), (dist, angle))| (x, y, dist, angle))
            .collect();

        // Sort by distance first, then clockwise by angle within each distance band
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
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cluster_x = target_x + dx;
                let cluster_y = target_y + dy;

                cluster.push((cluster_x, cluster_y));
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
    mut expand_events: EventWriter<ExpandGridCellsEvent>,
    grid: Res<Grid>,
    time: Res<Time>,
) {
    for mut scanner in &mut scanners {
        scanner.scan_timer.tick(time.delta());

        if scanner.scan_timer.just_finished() {
            if let Some((cluster, target_distance)) = scanner.find_next_cluster(&grid) {
                expand_events.send(ExpandGridCellsEvent {
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
    fn find_exploration_targets_deduplicates_by_coordinates() {
        let scanner = create_scanner_at_origin(1.0);
        // Create an L-shaped explored area where (1, 0) is adjacent to multiple explored tiles
        // Both (0, 0) and (1, 1) have (1, 0) as a neighbor
        let grid = create_grid_with_coordinates(&[(0, 0), (0, 1), (1, 1)]);

        let targets = scanner.find_exploration_targets(&grid);

        // Count how many times (1, 0) appears - should be exactly once
        let count_1_0 = targets
            .iter()
            .filter(|(x, y, _, _)| *x == 1 && *y == 0)
            .count();
        assert_eq!(
            count_1_0, 1,
            "(1, 0) should appear exactly once, found {count_1_0}"
        );

        // All coordinates should be unique
        let unique_coords: std::collections::HashSet<(i32, i32)> =
            targets.iter().map(|(x, y, _, _)| (*x, *y)).collect();
        assert_eq!(
            unique_coords.len(),
            targets.len(),
            "All coordinates should be unique"
        );
    }

    #[test]
    fn find_exploration_targets_sorts_by_distance_then_angle() {
        let mut scanner = create_scanner_at_origin(1.0);
        scanner.last_scan_angle = 0.0; // Start at north

        let grid = create_grid_with_coordinates(&[(0, 0)]);
        let targets = scanner.find_exploration_targets(&grid);

        // Verify targets are sorted by distance first, then by angle within each distance
        for i in 1..targets.len() {
            let (_, _, dist_prev, angle_prev) = targets[i - 1];
            let (_, _, dist_curr, angle_curr) = targets[i];

            if dist_prev == dist_curr {
                let angle_diff_prev = scanner.calculate_angle_diff(angle_prev);
                let angle_diff_curr = scanner.calculate_angle_diff(angle_curr);
                assert!(
                    angle_diff_curr >= angle_diff_prev - 0.001,
                    "Within same distance, targets should be sorted by angle: angle_diff at {} ({}) should be >= angle_diff at {} ({})",
                    i, angle_diff_curr, i - 1, angle_diff_prev
                );
            } else {
                assert!(
                    dist_curr >= dist_prev,
                    "Targets should be sorted by distance first: distance at {} ({}) should be >= distance at {} ({})",
                    i, dist_curr, i - 1, dist_prev
                );
            }
        }
    }

    #[test]
    fn find_exploration_targets_sweeps_clockwise_within_distance_band() {
        let mut scanner = Scanner::new(1.0, Position { x: 0, y: 0 });
        scanner.last_scan_angle = 0.0; // Looking north

        // Create a ring of explored tiles around the scanner
        // This creates unexplored targets at distance 2 in all directions
        let grid = create_grid_with_coordinates(&[
            (0, 0),
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ]);

        let targets = scanner.find_exploration_targets(&grid);

        // Get all targets at distance 2 (the outer ring)
        let dist_2_targets: Vec<_> = targets
            .iter()
            .filter(|(_, _, dist, _)| *dist == 2)
            .collect();

        // Verify they're sorted clockwise (by angle from north)
        for i in 1..dist_2_targets.len() {
            let angle_diff_prev = scanner.calculate_angle_diff(dist_2_targets[i - 1].3);
            let angle_diff_curr = scanner.calculate_angle_diff(dist_2_targets[i].3);
            assert!(
                angle_diff_curr >= angle_diff_prev - 0.001,
                "Within distance band, targets should sweep clockwise"
            );
        }
    }
}
