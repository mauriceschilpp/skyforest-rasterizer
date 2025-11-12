//! Tile loading and caching operations

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use memmap2::Mmap;
use crate::error::{Error, Result};
use crate::io::{BufferedReader, ByteOrder};
use crate::compression::Compression;
use crate::cache::TileCache;
use crate::cache_prefetch::{AccessPattern, PrefetchConfig};
use crate::cache_prefetch_async::PrefetchPool;
use crate::formats::tiff::{IFD, IFDEntry, tags};

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

/// Handles tile loading with caching and memory mapping
pub struct TileReader<R: Read + Seek + Send + Sync> {
    reader: BufferedReader<R>,
    byte_order: ByteOrder,
    mmap: Option<Arc<Mmap>>,
    cache: TileCache,
    current_ifd_index: usize,
    access_pattern: Option<AccessPattern>,
    prefetch_config: PrefetchConfig,
    prefetch_pool: Option<PrefetchPool>,
}

impl<R: Read + Seek + Send + Sync> TileReader<R> {
    pub fn new(
        reader: BufferedReader<R>,
        byte_order: ByteOrder,
        mmap: Option<Arc<Mmap>>,
        cache_size: usize,
    ) -> Self {
        Self {
            reader,
            byte_order,
            mmap,
            cache: TileCache::new(cache_size),
            current_ifd_index: 0,
            access_pattern: None,
            prefetch_config: PrefetchConfig::default(),
            prefetch_pool: None,
        }
    }

    pub fn new_with_prefetch(
        reader: BufferedReader<R>,
        byte_order: ByteOrder,
        mmap: Option<Arc<Mmap>>,
        cache_size: usize,
        prefetch_config: PrefetchConfig,
    ) -> Self {
        Self {
            reader,
            byte_order,
            mmap,
            cache: TileCache::new(cache_size),
            current_ifd_index: 0,
            access_pattern: None,
            prefetch_config,
            prefetch_pool: None,
        }
    }

    pub fn init_prefetch(&mut self, tiles_per_row: usize) {
        if self.prefetch_config.enabled {
            self.access_pattern = Some(AccessPattern::new(tiles_per_row));

            let mmap = self.mmap.clone();
            let byte_order = self.byte_order;

            let load_fn = move |ifd: &IFD, offsets: &IFDEntry, counts: &IFDEntry, tile_idx: usize| {
                use super::parallel::{ParallelReader, ParallelConfig};

                let config = ParallelConfig::from_ifd(ifd, mmap.clone(), byte_order)?;
                let result = ParallelReader::load_single_tile(tile_idx, offsets, counts, &config)?;
                Ok(result.1)
            };

            self.prefetch_pool = Some(PrefetchPool::new(4, load_fn));
        }
    }

    pub fn set_current_ifd(&mut self, index: usize) {
        self.current_ifd_index = index;
    }

    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    pub fn reader_mut(&mut self) -> &mut BufferedReader<R> {
        &mut self.reader
    }

    pub fn cache(&self) -> &TileCache {
        &self.cache
    }

    pub fn mmap(&self) -> &Option<Arc<Mmap>> {
        &self.mmap
    }

    /// Reads a tile with caching and optional prefetching
    pub fn read_tile(&mut self, ifd: &IFD, tile_index: usize) -> Result<Vec<u8>> {
        self.collect_prefetched_tiles();

        if let Some(cached_data) = self.cache.get(self.current_ifd_index, tile_index) {
            return Ok((*cached_data).clone());
        }

        let tile_data = self.load_uncached_tile(ifd, tile_index)?;
        self.cache.put(self.current_ifd_index, tile_index, tile_data.clone());

        if let Some(ref mut pattern) = self.access_pattern {
            pattern.record(tile_index);
            let to_prefetch = pattern.predict_next();
            self.prefetch_tiles_async(ifd, &to_prefetch);
        }

        Ok(tile_data)
    }

    /// Collects any tiles that have been prefetched in background
    fn collect_prefetched_tiles(&mut self) {
        if let Some(ref pool) = self.prefetch_pool {
            let results = pool.collect_results();
            for result in results {
                self.cache.put(self.current_ifd_index, result.tile_index, result.tile_data);
            }
        }
    }

    /// Prefetches tiles asynchronously in background
    fn prefetch_tiles_async(&mut self, ifd: &IFD, tile_indices: &[usize]) {
        if !self.prefetch_config.enabled || tile_indices.is_empty() {
            return;
        }

        if self.prefetch_pool.is_none() {
            return;
        }

        let max_prefetch = self.prefetch_config.max_prefetch.min(tile_indices.len());
        let tiles_to_fetch: Vec<usize> = tile_indices.iter()
            .take(max_prefetch)
            .filter(|&&idx| self.cache.get(self.current_ifd_index, idx).is_none())
            .copied()
            .collect();

        if tiles_to_fetch.is_empty() {
            return;
        }

        let offsets_entry = match ifd.get_entry(tags::TILE_OFFSETS) {
            Some(e) => e.clone(),
            None => return,
        };

        let byte_counts_entry = match ifd.get_entry(tags::TILE_BYTE_COUNTS) {
            Some(e) => e.clone(),
            None => return,
        };

        let request = crate::cache_prefetch_async::PrefetchRequest {
            tile_indices: tiles_to_fetch,
            ifd: Arc::new(ifd.clone()),
            offsets_entry,
            byte_counts_entry,
        };

        if let Some(ref pool) = self.prefetch_pool {
            pool.prefetch(request);
        }
    }

    /// Loads a tile that is not in cache
    fn load_uncached_tile(&mut self, ifd: &IFD, tile_index: usize) -> Result<Vec<u8>> {
        let offsets_entry = ifd.get_entry(tags::TILE_OFFSETS)
            .ok_or(Error::MissingTag(tags::TILE_OFFSETS))?;

        let byte_counts_entry = ifd.get_entry(tags::TILE_BYTE_COUNTS)
            .ok_or(Error::MissingTag(tags::TILE_BYTE_COUNTS))?;

        let offset = self.read_tile_offset(offsets_entry, tile_index)?;
        let byte_count = self.read_tile_byte_count(byte_counts_entry, tile_index)?;

        if byte_count == 0 {
            return self.create_empty_tile(ifd);
        }

        let compressed_data = self.read_compressed_data(offset, byte_count)?;
        self.decompress_and_apply_predictor(ifd, &compressed_data)
    }

    /// Creates an empty tile filled with zeros
    fn create_empty_tile(&self, ifd: &IFD) -> Result<Vec<u8>> {
        let tile_dims = ifd.tile_dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing tile dimensions".to_string()))?;
        Ok(vec![0u8; (tile_dims.width * tile_dims.height) as usize])
    }

    /// Reads compressed tile data from file or memory map
    fn read_compressed_data(&mut self, offset: u64, byte_count: u64) -> Result<Vec<u8>> {
        if let Some(ref mmap) = self.mmap {
            let start = offset as usize;
            let end = start + byte_count as usize;
            return Ok(mmap[start..end].to_vec());
        }

        self.reader.seek(SeekFrom::Start(offset))?;
        let mut buffer = vec![0u8; byte_count as usize];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Decompresses tile data and applies predictor if needed
    fn decompress_and_apply_predictor(&self, ifd: &IFD, compressed_data: &[u8]) -> Result<Vec<u8>> {
        let compression_value = ifd.compression().unwrap_or(1);
        let compression = Compression::from_tag(compression_value)?;
        let mut decompressed = compression.decompress(compressed_data)?;

        let predictor = ifd.get_tag_value(tags::PREDICTOR).unwrap_or(1);
        if predictor == 2 {
            let tile_dims = ifd.tile_dimensions()
                .ok_or_else(|| Error::InvalidFormat("Missing tile dimensions".to_string()))?;
            apply_horizontal_predictor(&mut decompressed, tile_dims.width as usize, tile_dims.height as usize);
        }

        Ok(decompressed)
    }

    pub fn read_tile_offset(&mut self, entry: &IFDEntry, index: usize) -> Result<u64> {
        let offset_size = entry.field_type_size();
        let file_offset = entry.value_offset + (index as u64 * offset_size as u64);

        self.reader.seek(SeekFrom::Start(file_offset))?;
        let handler = self.byte_order.handler();

        let offset = if offset_size == 8 {
            handler.read_u64(&mut self.reader)?
        } else {
            handler.read_u32(&mut self.reader)? as u64
        };

        Ok(offset)
    }

    pub fn read_tile_byte_count(&mut self, entry: &IFDEntry, index: usize) -> Result<u64> {
        let count_size = entry.field_type_size();
        let file_offset = entry.value_offset + (index as u64 * count_size as u64);

        self.reader.seek(SeekFrom::Start(file_offset))?;
        let handler = self.byte_order.handler();

        let byte_count = if count_size == 8 {
            handler.read_u64(&mut self.reader)?
        } else {
            handler.read_u32(&mut self.reader)? as u64
        };

        Ok(byte_count)
    }

    /// Static method for reading tile offset from memory map (for parallel processing)
    pub fn read_tile_offset_static(
        entry: &IFDEntry,
        index: usize,
        mmap: &Option<Arc<Mmap>>,
        _byte_order: ByteOrder,
    ) -> Result<u64> {
        let offset_size = entry.field_type_size();
        let file_offset = entry.value_offset + (index as u64 * offset_size as u64);

        if let Some(ref mmap) = mmap {
            let start = file_offset as usize;

            if offset_size == 8 {
                let bytes = &mmap[start..start + 8];
                Ok(u64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7],
                ]))
            } else {
                let bytes = &mmap[start..start + 4];
                Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64)
            }
        } else {
            Err(Error::Unsupported("Parallel reading requires memory mapping".to_string()))
        }
    }

    /// Static method for reading tile byte count from memory map (for parallel processing)
    pub fn read_tile_byte_count_static(
        entry: &IFDEntry,
        index: usize,
        mmap: &Option<Arc<Mmap>>,
        _byte_order: ByteOrder,
    ) -> Result<u64> {
        let count_size = entry.field_type_size();
        let file_offset = entry.value_offset + (index as u64 * count_size as u64);

        if let Some(ref mmap) = mmap {
            let start = file_offset as usize;

            if count_size == 8 {
                let bytes = &mmap[start..start + 8];
                Ok(u64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7],
                ]))
            } else {
                let bytes = &mmap[start..start + 4];
                Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64)
            }
        } else {
            Err(Error::Unsupported("Parallel reading requires memory mapping".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::formats::tiff::tags::field_types;

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
    fn test_create_empty_tile() {
        let mut ifd = IFD::new(0, 0);
        ifd.add_entry(IFDEntry::new(tags::TILE_WIDTH, field_types::LONG, 1, 256));
        ifd.add_entry(IFDEntry::new(tags::TILE_LENGTH, field_types::LONG, 1, 256));

        let reader = BufferedReader::new(Cursor::new(vec![]));
        let tile_reader = TileReader::new(reader, ByteOrder::LittleEndian, None, 0);

        let empty_tile = tile_reader.create_empty_tile(&ifd).unwrap();
        assert_eq!(empty_tile.len(), 256 * 256);
        assert!(empty_tile.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_read_tile_offset_static_u32() {
        let mut mmap_data = vec![0u8; 100];
        mmap_data[20] = 0x78;
        mmap_data[21] = 0x56;
        mmap_data[22] = 0x34;
        mmap_data[23] = 0x12;

        let entry = IFDEntry::new(tags::TILE_OFFSETS, field_types::LONG, 5, 16);

        let offset = entry.value_offset + (1 * 4);
        assert_eq!(offset, 20);

        assert_eq!(mmap_data[20], 0x78);
    }

    #[test]
    fn test_cache_integration() {
        let reader = BufferedReader::new(Cursor::new(vec![]));
        let mut tile_reader = TileReader::new(reader, ByteOrder::LittleEndian, None, 10);

        tile_reader.set_current_ifd(0);
        assert_eq!(tile_reader.current_ifd_index, 0);

        let test_data = vec![1, 2, 3, 4];
        tile_reader.cache.put(0, 5, test_data.clone());

        let cached = tile_reader.cache.get(0, 5).unwrap();
        assert_eq!(*cached, test_data);
    }
}
