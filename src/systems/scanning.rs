use std::collections::HashSet;

use bevy::prelude::*;

use crate::{grid::{ExpandGridCellsEvent, Grid, Position}, structures::Building};

#[derive(Component)]
pub struct Scanner {
    pub scanned_cells: HashSet<(i32, i32)>,  // Cells this scanner has revealed
    pub max_scan_distance: i32,              // Maximum distance from scanner position
    pub scan_timer: Timer,                   // Timer for scanning intervals
    pub position: Position,                  // Cached position for efficiency
}

impl Scanner {
    pub fn new(max_scan_distance: i32, scan_interval_secs: f32, position: Position) -> Self {
        let mut scanned_cells = HashSet::new();
        // Scanner starts with its own position scanned
        scanned_cells.insert((position.x, position.y));
        
        Self {
            scanned_cells,
            max_scan_distance,
            scan_timer: Timer::from_seconds(scan_interval_secs, TimerMode::Repeating),
            position,
        }
    }
    
    /// Synchronize scanner state with current grid - call this after creating scanner
    pub fn sync_with_grid(&mut self, grid: &Grid) {
        // Add all existing grid coordinates within scan range to scanned_cells
        for &(x, y) in &grid.valid_coordinates {
            let distance = (x - self.position.x).abs().max((y - self.position.y).abs());
            if distance <= self.max_scan_distance {
                self.scanned_cells.insert((x, y));
            }
        }
    }
    
    pub fn is_complete(&self) -> bool {
        // Complete when we've scanned a reasonable area (e.g., all cells within max distance)
        let total_possible = ((self.max_scan_distance * 2 + 1) as usize).pow(2);
        self.scanned_cells.len() >= total_possible
    }
    
    pub fn find_next_cluster(&self, grid: &Grid) -> Option<Vec<(i32, i32)>> {
        let mut best_cluster = None;
        let mut best_score = -1;
        
        // Search in expanding rings from scanner position
        for distance in 1..=self.max_scan_distance {
            let mut found_valid_cluster = false;
            
            // Check all positions at this distance
            for dy in -distance..=distance {
                for dx in -distance..=distance {
                    // Only check positions at the current distance boundary
                    let dist_from_edge = dx.abs().max(dy.abs());
                    if dist_from_edge != distance {
                        continue;
                    }
                    
                    let center_x = self.position.x + dx;
                    let center_y = self.position.y + dy;
                    
                    let cluster = self.get_3x3_cluster(center_x, center_y);
                    let score = self.score_cluster(&cluster, grid);
                    
                    if score > best_score {
                        best_score = score;
                        best_cluster = Some(cluster);
                        found_valid_cluster = true;
                    }
                }
            }
            
            // Only break if we found a cluster with unscanned cells
            if found_valid_cluster && best_score > 0 {
                break;
            }
        }
        
        best_cluster
    }
    
    fn get_3x3_cluster(&self, center_x: i32, center_y: i32) -> Vec<(i32, i32)> {
        let mut cluster = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                cluster.push((center_x + dx, center_y + dy));
            }
        }
        cluster
    }
    
    fn score_cluster(&self, cluster: &[(i32, i32)], grid: &Grid) -> i32 {
        let mut unscanned_count = 0;
        let mut adjacent_to_scanned = false;
        let mut valid_cells_in_cluster = 0;
        
        for &(x, y) in cluster {
            // Check if within max scan distance
            let distance = (x - self.position.x).abs().max((y - self.position.y).abs());
            if distance > self.max_scan_distance {
                return -1; // Invalid cluster - outside scan range
            }
            
            // Check if cell is already revealed in the grid OR marked as scanned by this scanner
            let already_revealed = grid.valid_coordinates.contains(&(x, y)) || 
                                 self.scanned_cells.contains(&(x, y));
            
            if !already_revealed {
                unscanned_count += 1;
            } else {
                valid_cells_in_cluster += 1;
            }
            
            // Check if adjacent to any scanned cell (either in grid or scanner's memory)
            if !adjacent_to_scanned {
                for (adj_x, adj_y) in [(x+1, y), (x-1, y), (x, y+1), (x, y-1)] {
                    let adj_revealed = grid.valid_coordinates.contains(&(adj_x, adj_y)) || 
                                     self.scanned_cells.contains(&(adj_x, adj_y));
                    if adj_revealed {
                        adjacent_to_scanned = true;
                        break;
                    }
                }
            }
        }
        
        // Return -1 if no unscanned cells (completely revealed)
        if unscanned_count == 0 {
            return -1;
        }
        
        // Prioritize clusters that are adjacent to already scanned areas
        let base_score = unscanned_count * 10;
        
        if adjacent_to_scanned {
            // Bonus for continuity
            base_score + 100
        } else {
            // Lower priority for isolated clusters
            base_score
        }
    }
    
    pub fn mark_cluster_scanned(&mut self, cluster: &[(i32, i32)]) {
        for &coord in cluster {
            self.scanned_cells.insert(coord);
        }
    }
}

pub fn initialize_new_scanners(
    mut scanners: Query<&mut Scanner, Added<Scanner>>,
    grid: Res<Grid>,
) {
    for mut scanner in scanners.iter_mut() {
        scanner.sync_with_grid(&grid);
        println!("Scanner at ({}, {}) synchronized with {} existing grid cells", 
                 scanner.position.x, scanner.position.y, scanner.scanned_cells.len());
    }
}

pub fn handle_progressive_scanning(
    mut scanners: Query<&mut Scanner, With<Building>>,
    mut expand_events: EventWriter<ExpandGridCellsEvent>,
    grid: Res<Grid>,
    time: Res<Time>,
) {
    for mut scanner in scanners.iter_mut() {
        if scanner.is_complete() {
            continue;
        }
        
        scanner.scan_timer.tick(time.delta());
        
        if scanner.scan_timer.just_finished() {
            if let Some(cluster) = scanner.find_next_cluster(&grid) {
                // Send expand event for the 3x3 cluster
                expand_events.send(ExpandGridCellsEvent {
                    coordinates: cluster.clone(),
                });
                
                // Mark cluster as scanned
                scanner.mark_cluster_scanned(&cluster);
                
                println!("Scanner at ({}, {}) revealed 3x3 cluster with {} cells", 
                         scanner.position.x, scanner.position.y, cluster.len());
            }
        }
    }
}