use bevy::prelude::*;

use crate::{grid::{ExpandGridCellsEvent, Grid, Position}, structures::Building};

#[derive(Component)]
pub struct Scanner {
    pub max_scan_distance: i32,
    pub scan_timer: Timer,
    pub position: Position,
    pub last_scan_angle: f32, // Track progress around perimeter
}

impl Scanner {
    pub fn new(max_scan_distance: i32, scan_interval_secs: f32, position: Position) -> Self {
        Self {
            max_scan_distance,
            scan_timer: Timer::from_seconds(scan_interval_secs, TimerMode::Repeating),
            position,
            last_scan_angle: 0.0, // Start from north
        }
    }
    
    /// Calculate angle from scanner to point, with North=0, increasing clockwise
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
    
    /// Find unexplored tiles adjacent to explored areas, sorted by distance then angle
    fn find_exploration_targets(&self, grid: &Grid) -> Vec<(i32, i32, i32, f32)> {
        let mut targets = Vec::new();
        
        // Check all explored tiles for unexplored neighbors
        for &(x, y) in &grid.valid_coordinates {
            let distance_from_scanner = (x - self.position.x).abs().max((y - self.position.y).abs());
            
            // Only consider tiles within reasonable distance of scanner
            if distance_from_scanner > self.max_scan_distance {
                continue;
            }
            
            // Check 4-directional neighbors for unexplored areas
            let neighbors = [(x+1, y), (x-1, y), (x, y+1), (x, y-1)];
            for (nx, ny) in neighbors {
                let neighbor_distance = (nx - self.position.x).abs().max((ny - self.position.y).abs());
                
                // Neighbor must be within scanner range and unexplored
                if neighbor_distance <= self.max_scan_distance && 
                   !grid.valid_coordinates.contains(&(nx, ny)) {
                    
                    let angle = self.calculate_angle(nx, ny);
                    targets.push((nx, ny, neighbor_distance, angle));
                }
            }
        }
        
        // Remove duplicates
        targets.sort_by(|a, b| {
            // Primary sort: distance (closer is better)
            let distance_cmp = a.2.cmp(&b.2);
            if distance_cmp != std::cmp::Ordering::Equal {
                return distance_cmp;
            }
            
            // Secondary sort: angle progression from last scan position
            let angle_diff_a = self.calculate_angle_diff(a.3);
            let angle_diff_b = self.calculate_angle_diff(b.3);
            angle_diff_a.partial_cmp(&angle_diff_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        targets.dedup();
        
        // Sort by distance first (closer tiles prioritized), then by angle progression
        targets.sort_by(|a, b| {
            // Primary sort: distance (closer is better)
            let distance_cmp = a.2.cmp(&b.2);
            if distance_cmp != std::cmp::Ordering::Equal {
                return distance_cmp;
            }
            
            // Secondary sort: angle progression from last scan position
            let angle_diff_a = self.calculate_angle_diff(a.3);
            let angle_diff_b = self.calculate_angle_diff(b.3);
            angle_diff_a.partial_cmp(&angle_diff_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        targets
    }
    
    /// Calculate the angular difference from last scan position, preferring clockwise progression
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
    
    /// Find next cluster to reveal, prioritizing systematic exploration
    pub fn find_next_cluster(&mut self, grid: &Grid) -> Option<Vec<(i32, i32)>> {
        let targets = self.find_exploration_targets(grid);
        
        if targets.is_empty() {
            println!("Scanner at ({}, {}) found no exploration targets", 
                     self.position.x, self.position.y);
            return None;
        }
        
        // Take the best target (closest distance, best angle progression)
        let (target_x, target_y, distance, angle) = targets[0];
        
        // Update our scan progression
        self.last_scan_angle = angle;
        
        // Create a 3x3 cluster centered on the target
        let mut cluster = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cluster_x = target_x + dx;
                let cluster_y = target_y + dy;
                
                // Only include tiles within scanner range
                let cluster_distance = (cluster_x - self.position.x).abs()
                    .max((cluster_y - self.position.y).abs());
                
                if cluster_distance <= self.max_scan_distance {
                    cluster.push((cluster_x, cluster_y));
                }
            }
        }
        
        println!("Scanner at ({}, {}) targeting ({}, {}) at distance {} angle {:.2} - cluster size {}", 
                 self.position.x, self.position.y, target_x, target_y, distance, angle, cluster.len());
        
        Some(cluster)
    }
}

pub fn handle_progressive_scanning(
    mut scanners: Query<&mut Scanner, With<Building>>,
    mut expand_events: EventWriter<ExpandGridCellsEvent>,
    grid: Res<Grid>,
    time: Res<Time>,
) {
    for mut scanner in scanners.iter_mut() {
        scanner.scan_timer.tick(time.delta());
        
        if scanner.scan_timer.just_finished() {
            if let Some(cluster) = scanner.find_next_cluster(&grid) {
                expand_events.send(ExpandGridCellsEvent {
                    coordinates: cluster,
                });
            }
        }
    }
}