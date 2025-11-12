/// Tile prefetching and access pattern prediction

use std::collections::VecDeque;

/// Maximum history size for pattern detection
const HISTORY_SIZE: usize = 8;

/// Minimum pattern length to trigger prefetching
const MIN_PATTERN_LENGTH: usize = 3;

/// Access pattern tracker for intelligent prefetching
pub struct AccessPattern {
    history: VecDeque<usize>,
    tiles_per_row: usize,
}

impl AccessPattern {
    /// Creates a new access pattern tracker
    pub fn new(tiles_per_row: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(HISTORY_SIZE),
            tiles_per_row,
        }
    }

    /// Records a tile access
    pub fn record(&mut self, tile_index: usize) {
        if self.history.len() >= HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(tile_index);
    }

    /// Predicts which tiles should be prefetched
    pub fn predict_next(&self) -> Vec<usize> {
        if self.history.len() < MIN_PATTERN_LENGTH {
            return vec![];
        }

        if let Some(tiles) = self.detect_sequential() {
            return tiles;
        }

        if let Some(tiles) = self.detect_raster_scan() {
            return tiles;
        }

        if let Some(tiles) = self.detect_spatial_locality() {
            return tiles;
        }

        vec![]
    }

    /// Detects sequential access pattern (0, 1, 2, 3...)
    fn detect_sequential(&self) -> Option<Vec<usize>> {
        if self.history.len() < MIN_PATTERN_LENGTH {
            return None;
        }

        let mut is_sequential = true;
        for i in 1..self.history.len() {
            if self.history[i] != self.history[i - 1] + 1 {
                is_sequential = false;
                break;
            }
        }

        if is_sequential {
            let last = *self.history.back().unwrap();
            return Some((1..=16).map(|i| last + i).collect());
        }

        None
    }

    /// Detects raster scan pattern (row by row)
    fn detect_raster_scan(&self) -> Option<Vec<usize>> {
        if self.history.len() < MIN_PATTERN_LENGTH {
            return None;
        }

        let last = *self.history.back().unwrap();
        let second_last = self.history[self.history.len() - 2];

        let diff = if last > second_last {
            last - second_last
        } else {
            return None;
        };

        if diff == 1 {
            return Some((1..=16).map(|i| last + i).collect());
        }

        if diff == self.tiles_per_row {
            return Some((1..=16).map(|i| last + i * self.tiles_per_row).collect());
        }

        None
    }

    /// Predicts neighboring tiles based on spatial locality
    fn detect_spatial_locality(&self) -> Option<Vec<usize>> {
        if self.history.is_empty() {
            return None;
        }

        let last = *self.history.back().unwrap();
        let tile_x = last % self.tiles_per_row;
        let _tile_y = last / self.tiles_per_row;

        let mut neighbors = Vec::new();

        // Prefetch 4x4 grid of surrounding tiles (16 tiles total)
        for dy in 0..4 {
            for dx in 0..4 {
                if dx == 0 && dy == 0 {
                    continue; // Skip the current tile
                }
                if tile_x + dx < self.tiles_per_row {
                    neighbors.push(last + dy * self.tiles_per_row + dx);
                }
            }
        }

        Some(neighbors)
    }
}

/// Configuration for prefetching behavior
pub struct PrefetchConfig {
    /// Enable prefetching
    pub enabled: bool,

    /// Maximum number of tiles to prefetch
    pub max_prefetch: usize,

    /// Enable spatial locality prefetching
    pub spatial: bool,

    /// Enable sequential pattern detection
    pub sequential: bool,

    /// Enable raster scan detection
    pub raster_scan: bool,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_prefetch: 8,
            spatial: true,
            sequential: true,
            raster_scan: true,
        }
    }
}

impl PrefetchConfig {
    /// Creates a disabled prefetch configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_prefetch: 0,
            spatial: false,
            sequential: false,
            raster_scan: false,
        }
    }

    /// Creates an aggressive prefetch configuration
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            max_prefetch: 16,
            spatial: true,
            sequential: true,
            raster_scan: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_detection() {
        let mut pattern = AccessPattern::new(100);

        pattern.record(10);
        pattern.record(11);
        pattern.record(12);

        let predicted = pattern.predict_next();
        assert_eq!(predicted.len(), 16);
        assert_eq!(predicted[0], 13);
        assert_eq!(predicted[15], 28);
    }

    #[test]
    fn test_raster_scan_detection() {
        let mut pattern = AccessPattern::new(10);

        pattern.record(0);
        pattern.record(1);
        pattern.record(2);
        pattern.record(10);

        let predicted = pattern.predict_next();
        assert!(predicted.contains(&20) || predicted.contains(&11));
    }

    #[test]
    fn test_spatial_locality() {
        let mut pattern = AccessPattern::new(10);

        pattern.record(5);
        pattern.record(47);
        pattern.record(23);

        let predicted = pattern.predict_next();
        assert!(!predicted.is_empty());
        assert!(predicted.contains(&24) || predicted.contains(&33));
    }

    #[test]
    fn test_no_pattern() {
        let mut pattern = AccessPattern::new(10);

        pattern.record(5);
        pattern.record(47);
        pattern.record(23);
        pattern.record(91);

        let predicted = pattern.predict_next();
        assert!(!predicted.is_empty());
    }
}
