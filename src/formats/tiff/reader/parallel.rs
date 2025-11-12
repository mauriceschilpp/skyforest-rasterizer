//! Parallel tile processing operations

use std::sync::Arc;
use memmap2::Mmap;
use rayon::prelude::*;
use crate::error::{Error, Result};
use crate::io::ByteOrder;
use crate::compression::Compression;
use crate::cache::TileCache;
use crate::formats::tiff::{IFD, IFDEntry, tags};
use super::tiles::TileReader;

/// Configuration for parallel tile processing
pub struct ParallelConfig {
    pub compression_value: u64,
    pub predictor: u64,
    pub tile_width: u64,
    pub tile_height: u64,
    pub mmap: Option<Arc<Mmap>>,
    pub byte_order: ByteOrder,
}

impl ParallelConfig {
    /// Creates configuration from IFD and reader state
    pub fn from_ifd(ifd: &IFD, mmap: Option<Arc<Mmap>>, byte_order: ByteOrder) -> Result<Self> {
        let tile_dims = ifd.tile_dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing tile dimensions".to_string()))?;

        Ok(Self {
            compression_value: ifd.compression().unwrap_or(1),
            predictor: ifd.get_tag_value(tags::PREDICTOR).unwrap_or(1),
            tile_width: tile_dims.width,
            tile_height: tile_dims.height,
            mmap,
            byte_order,
        })
    }
}

/// Handles parallel tile reading operations
pub struct ParallelReader;

impl ParallelReader {
    /// Reads multiple tiles in parallel
    ///
    /// This method leverages rayon to decompress multiple tiles simultaneously.
    /// Requires memory mapping to be enabled.
    pub fn read_tiles_parallel(
        ifd: &IFD,
        tile_indices: &[usize],
        cache: &TileCache,
        current_ifd_index: usize,
        mmap: &Option<Arc<Mmap>>,
        byte_order: ByteOrder,
    ) -> Result<Vec<Vec<u8>>> {
        let uncached_indices = Self::find_uncached_indices(tile_indices, cache, current_ifd_index);

        if !uncached_indices.is_empty() {
            Self::load_uncached_tiles_parallel(
                ifd,
                &uncached_indices,
                cache,
                current_ifd_index,
                mmap,
                byte_order,
            )?;
        }

        Self::collect_tiles_from_cache(tile_indices, cache, current_ifd_index)
    }

    /// Reads multiple tiles in parallel without caching
    ///
    /// This method loads tiles directly and returns them without cache interaction.
    /// Useful for batch operations where tiles may exceed cache capacity.
    pub fn read_tiles_parallel_direct(
        ifd: &IFD,
        tile_indices: &[usize],
        mmap: &Option<Arc<Mmap>>,
        byte_order: ByteOrder,
    ) -> Result<Vec<Vec<u8>>> {
        let offsets_entry = ifd.get_entry(tags::TILE_OFFSETS)
            .ok_or(Error::MissingTag(tags::TILE_OFFSETS))?;
        let byte_counts_entry = ifd.get_entry(tags::TILE_BYTE_COUNTS)
            .ok_or(Error::MissingTag(tags::TILE_BYTE_COUNTS))?;

        let config = ParallelConfig::from_ifd(ifd, mmap.clone(), byte_order)?;

        let tile_results: Vec<_> = tile_indices
            .par_iter()
            .map(|&tile_idx| {
                Self::load_single_tile(tile_idx, offsets_entry, byte_counts_entry, &config)
            })
            .collect();

        let mut tiles = Vec::with_capacity(tile_indices.len());
        for result in tile_results {
            let (_, data) = result?;
            tiles.push(data);
        }

        Ok(tiles)
    }

    /// Identifies which tiles are not in cache
    fn find_uncached_indices(
        tile_indices: &[usize],
        cache: &TileCache,
        ifd_index: usize,
    ) -> Vec<usize> {
        tile_indices
            .iter()
            .filter(|&&idx| cache.get(ifd_index, idx).is_none())
            .copied()
            .collect()
    }

    /// Loads uncached tiles in parallel and puts them in cache
    fn load_uncached_tiles_parallel(
        ifd: &IFD,
        uncached_indices: &[usize],
        cache: &TileCache,
        ifd_index: usize,
        mmap: &Option<Arc<Mmap>>,
        byte_order: ByteOrder,
    ) -> Result<()> {
        let offsets_entry = ifd.get_entry(tags::TILE_OFFSETS)
            .ok_or(Error::MissingTag(tags::TILE_OFFSETS))?;
        let byte_counts_entry = ifd.get_entry(tags::TILE_BYTE_COUNTS)
            .ok_or(Error::MissingTag(tags::TILE_BYTE_COUNTS))?;

        let config = ParallelConfig::from_ifd(ifd, mmap.clone(), byte_order)?;

        let tile_results: Vec<_> = uncached_indices
            .par_iter()
            .map(|&tile_idx| {
                Self::load_single_tile(tile_idx, offsets_entry, byte_counts_entry, &config)
            })
            .collect();

        for result in tile_results {
            match result {
                Ok((tile_idx, data)) => {
                    cache.put(ifd_index, tile_idx, data);
                },
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Loads a single tile in parallel context
    pub fn load_single_tile(
        tile_idx: usize,
        offsets_entry: &IFDEntry,
        byte_counts_entry: &IFDEntry,
        config: &ParallelConfig,
    ) -> Result<(usize, Vec<u8>)> {
        let offset = TileReader::<std::fs::File>::read_tile_offset_static(
            offsets_entry,
            tile_idx,
            &config.mmap,
            config.byte_order,
        )?;

        let byte_count = TileReader::<std::fs::File>::read_tile_byte_count_static(
            byte_counts_entry,
            tile_idx,
            &config.mmap,
            config.byte_order,
        )?;

        if byte_count == 0 {
            return Ok((
                tile_idx,
                vec![0u8; (config.tile_width * config.tile_height) as usize],
            ));
        }

        let decompressed = Self::decompress_tile(tile_idx, offset, byte_count, config)?;
        Ok((tile_idx, decompressed))
    }

    /// Decompresses a tile in parallel context
    fn decompress_tile(
        tile_idx: usize,
        offset: u64,
        byte_count: u64,
        config: &ParallelConfig,
    ) -> Result<Vec<u8>> {
        let mmap = config.mmap.as_ref()
            .ok_or_else(|| Error::Unsupported("Parallel reading requires memory mapping".to_string()))?;

        let start = offset as usize;
        let end = start + byte_count as usize;

        if end > mmap.len() {
            return Err(Error::OutOfBounds(format!(
                "Tile {} data range {}-{} exceeds file size {}",
                tile_idx, start, end, mmap.len()
            )));
        }

        let compressed = &mmap[start..end];

        let compression = Compression::from_tag(config.compression_value)?;
        let mut decompressed = compression.decompress(compressed)?;

        if config.predictor == 2 {
            apply_horizontal_predictor(
                &mut decompressed,
                config.tile_width as usize,
                config.tile_height as usize,
            );
        }

        Ok(decompressed)
    }

    /// Collects requested tiles from cache
    fn collect_tiles_from_cache(
        tile_indices: &[usize],
        cache: &TileCache,
        ifd_index: usize,
    ) -> Result<Vec<Vec<u8>>> {
        tile_indices
            .iter()
            .map(|&idx| {
                cache
                    .get(ifd_index, idx)
                    .map(|data| (*data).clone())
                    .ok_or_else(|| Error::InvalidFormat(format!("Failed to read tile {}", idx)))
            })
            .collect()
    }
}

/// Apply horizontal differencing predictor with SIMD optimization for x86_64
#[cfg(target_arch = "x86_64")]
fn apply_horizontal_predictor(data: &mut [u8], width: usize, height: usize) {
    use std::arch::x86_64::*;

    for row in 0..height {
        let start = row * width;
        let end = (start + width).min(data.len());
        let row_data = &mut data[start..end];

        if row_data.is_empty() {
            continue;
        }

        let mut prev = row_data[0];
        let mut i = 1;

        unsafe {
            while i + 16 <= row_data.len() {
                let mut result = [0u8; 16];
                for j in 0..16 {
                    let val = row_data[i + j];
                    result[j] = val.wrapping_add(prev);
                    prev = result[j];
                }

                _mm_storeu_si128(row_data.as_mut_ptr().add(i) as *mut __m128i,
                                _mm_loadu_si128(result.as_ptr() as *const __m128i));
                i += 16;
            }
        }

        while i < row_data.len() {
            row_data[i] = row_data[i].wrapping_add(prev);
            prev = row_data[i];
            i += 1;
        }
    }
}

/// Apply horizontal differencing predictor with SIMD optimization for ARM
#[cfg(target_arch = "aarch64")]
fn apply_horizontal_predictor(data: &mut [u8], width: usize, height: usize) {
    use std::arch::aarch64::*;

    for row in 0..height {
        let start = row * width;
        let end = (start + width).min(data.len());
        let row_data = &mut data[start..end];

        if row_data.is_empty() {
            continue;
        }

        let mut prev = row_data[0];
        let mut i = 1;

        unsafe {
            while i + 16 <= row_data.len() {
                let mut result = [0u8; 16];
                for j in 0..16 {
                    let val = row_data[i + j];
                    result[j] = val.wrapping_add(prev);
                    prev = result[j];
                }

                vst1q_u8(row_data.as_mut_ptr().add(i), vld1q_u8(result.as_ptr()));
                i += 16;
            }
        }

        while i < row_data.len() {
            row_data[i] = row_data[i].wrapping_add(prev);
            prev = row_data[i];
            i += 1;
        }
    }
}

/// Apply horizontal differencing predictor (scalar fallback)
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn apply_horizontal_predictor(data: &mut [u8], width: usize, height: usize) {
    for row in 0..height {
        let start = row * width;
        let end = (start + width).min(data.len());

        for i in (start + 1)..end {
            data[i] = data[i].wrapping_add(data[i - 1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_horizontal_predictor() {
        let mut data = vec![1, 2, 3, 4, 5, 6];
        apply_horizontal_predictor(&mut data, 3, 2);

        assert_eq!(data[0], 1);
        assert_eq!(data[1], 3);
        assert_eq!(data[2], 6);
        assert_eq!(data[3], 4);
        assert_eq!(data[4], 9);
        assert_eq!(data[5], 15);
    }

    #[test]
    fn test_find_uncached_indices() {
        let cache = TileCache::new(10);
        cache.put(0, 1, vec![1, 2, 3]);
        cache.put(0, 3, vec![4, 5, 6]);

        let tile_indices = vec![0, 1, 2, 3, 4];
        let uncached = ParallelReader::find_uncached_indices(&tile_indices, &cache, 0);

        assert_eq!(uncached.len(), 3);
        assert!(uncached.contains(&0));
        assert!(uncached.contains(&2));
        assert!(uncached.contains(&4));
        assert!(!uncached.contains(&1));
        assert!(!uncached.contains(&3));
    }

    #[test]
    fn test_parallel_config_from_ifd() {
        use crate::formats::tiff::{IFD, IFDEntry};
        use crate::formats::tiff::tags::field_types;

        let mut ifd = IFD::new(0, 0);
        ifd.add_entry(IFDEntry::new(tags::TILE_WIDTH, field_types::LONG, 1, 256));
        ifd.add_entry(IFDEntry::new(tags::TILE_LENGTH, field_types::LONG, 1, 256));
        ifd.add_entry(IFDEntry::new(tags::COMPRESSION, field_types::SHORT, 1, 1));

        let config = ParallelConfig::from_ifd(&ifd, None, ByteOrder::LittleEndian).unwrap();

        assert_eq!(config.tile_width, 256);
        assert_eq!(config.tile_height, 256);
        assert_eq!(config.compression_value, 1);
        assert_eq!(config.predictor, 1);
    }

    #[test]
    fn test_collect_tiles_from_cache() {
        let cache = TileCache::new(10);
        cache.put(0, 0, vec![1, 2, 3]);
        cache.put(0, 1, vec![4, 5, 6]);
        cache.put(0, 2, vec![7, 8, 9]);

        let tile_indices = vec![0, 2];
        let result = ParallelReader::collect_tiles_from_cache(&tile_indices, &cache, 0).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![1, 2, 3]);
        assert_eq!(result[1], vec![7, 8, 9]);
    }

    #[test]
    fn test_collect_tiles_from_cache_missing() {
        let cache = TileCache::new(10);
        cache.put(0, 0, vec![1, 2, 3]);

        let tile_indices = vec![0, 1];
        let result = ParallelReader::collect_tiles_from_cache(&tile_indices, &cache, 0);

        assert!(result.is_err());
    }
}
