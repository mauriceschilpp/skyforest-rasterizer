/// Lock-free tile caching for efficient raster access

use std::sync::Arc;
use dashmap::DashMap;
use crossbeam::queue::SegQueue;

/// Lock-free LRU tile cache for storing decompressed tile data
pub struct TileCache {
    cache: Arc<DashMap<(usize, usize), Arc<Vec<u8>>>>,
    lru: Arc<SegQueue<(usize, usize)>>,
    max_tiles: usize,
}

impl TileCache {
    /// Creates a new lock-free tile cache
    ///
    /// # Arguments
    /// * `max_tiles` - Maximum number of tiles to cache (default: 256)
    pub fn new(max_tiles: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            lru: Arc::new(SegQueue::new()),
            max_tiles: max_tiles.max(1),
        }
    }

    /// Gets a tile from the cache (lock-free)
    ///
    /// # Arguments
    /// * `ifd_index` - IFD index
    /// * `tile_index` - Tile index within the IFD
    pub fn get(&self, ifd_index: usize, tile_index: usize) -> Option<Arc<Vec<u8>>> {
        let key = (ifd_index, tile_index);

        if let Some(entry) = self.cache.get(&key) {
            self.lru.push(key);
            return Some(Arc::clone(entry.value()));
        }

        None
    }

    /// Puts a tile into the cache (lock-free)
    ///
    /// # Arguments
    /// * `ifd_index` - IFD index
    /// * `tile_index` - Tile index within the IFD
    /// * `data` - Decompressed tile data
    pub fn put(&self, ifd_index: usize, tile_index: usize, data: Vec<u8>) {
        let key = (ifd_index, tile_index);
        let data_arc = Arc::new(data);

        while self.cache.len() >= self.max_tiles {
            if let Some(old_key) = self.lru.pop() {
                self.cache.remove(&old_key);
            } else {
                break;
            }
        }

        self.cache.insert(key, data_arc);
        self.lru.push(key);
    }

    /// Clears the cache
    pub fn clear(&self) {
        self.cache.clear();

        while self.lru.pop().is_some() {}
    }

    /// Returns the current number of cached tiles
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns whether the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Returns cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_bytes: usize = self.cache
            .iter()
            .map(|entry| entry.value().len())
            .sum();

        CacheStats {
            tile_count: self.cache.len(),
            total_bytes,
            max_tiles: self.max_tiles,
        }
    }
}

impl Clone for TileCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            lru: Arc::clone(&self.lru),
            max_tiles: self.max_tiles,
        }
    }
}

impl Default for TileCache {
    fn default() -> Self {
        Self::new(256)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of tiles currently in cache
    pub tile_count: usize,
    /// Total bytes used by cache
    pub total_bytes: usize,
    /// Maximum number of tiles
    pub max_tiles: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = TileCache::new(2);

        let data1 = vec![1, 2, 3];
        cache.put(0, 0, data1.clone());

        assert_eq!(cache.len(), 1);
        assert_eq!(*cache.get(0, 0).unwrap(), data1);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = TileCache::new(2);

        cache.put(0, 0, vec![1]);
        cache.put(0, 1, vec![2]);
        cache.put(0, 2, vec![3]);

        assert!(cache.len() <= 2);
    }

    #[test]
    fn test_cache_concurrent_access() {
        use std::thread;

        let cache = TileCache::new(100);

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cache_clone = cache.clone();
                thread::spawn(move || {
                    for j in 0..100 {
                        cache_clone.put(0, i * 100 + j, vec![i as u8, j as u8]);
                        let _ = cache_clone.get(0, i * 100 + j);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(cache.len() <= 100);
    }

    #[test]
    fn test_cache_clear() {
        let cache = TileCache::new(10);

        cache.put(0, 0, vec![1]);
        cache.put(0, 1, vec![2]);

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let cache = TileCache::new(10);

        cache.put(0, 0, vec![1, 2, 3]);
        cache.put(0, 1, vec![4, 5]);

        let stats = cache.stats();
        assert_eq!(stats.tile_count, 2);
        assert_eq!(stats.total_bytes, 5);
        assert_eq!(stats.max_tiles, 10);
    }
}
