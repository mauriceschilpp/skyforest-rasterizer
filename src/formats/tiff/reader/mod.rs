//! TIFF reader modules

pub mod tags;
pub mod tiles;
pub mod pixels;
pub mod parallel;

use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use memmap2::Mmap;
use crate::error::{Error, Result};
use crate::io::{BufferedReader, ByteOrder};
use crate::formats::tiff::{Tiff, IFD, IFDEntry, TIFF_MAGIC, BIGTIFF_MAGIC};

use self::tags::TagReader;
use self::tiles::TileReader;
use self::pixels::PixelReader;
use self::parallel::ParallelReader;

pub use crate::cache_prefetch::PrefetchConfig;

/// TIFF file reader with modular architecture
pub struct TiffReader {
    tile_reader: TileReader<File>,
    byte_order: ByteOrder,
    is_big_tiff: bool,
    current_ifd_index: usize,
}

impl TiffReader {
    /// Opens a TIFF file for reading with default options
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_options(path, true, 256)
    }

    /// Enable prefetching for improved sequential access performance
    pub fn enable_prefetch(&mut self, ifd: &IFD) {
        if let Some(tile_dims) = ifd.tile_dimensions() {
            let image_dims = ifd.dimensions().unwrap_or(tile_dims);
            let tiles_per_row = ((image_dims.width + tile_dims.width - 1) / tile_dims.width) as usize;
            self.tile_reader.init_prefetch(tiles_per_row);
        }
    }


    /// Helper: Reads first IFD offset
    fn read_first_ifd_offset(&mut self) -> Result<u64> {
        let handler = self.byte_order.handler();
        let reader = self.tile_reader.reader_mut();

        if self.is_big_tiff {
            Ok(handler.read_u64(reader)?)
        } else {
            Ok(handler.read_u32(reader)? as u64)
        }
    }

    /// Helper: Reads next IFD offset from current position
    fn read_next_ifd_offset(&mut self, entries_end: u64) -> Result<u64> {
        let handler = self.byte_order.handler();
        let reader = self.tile_reader.reader_mut();

        reader.seek(SeekFrom::Start(entries_end))?;

        if self.is_big_tiff {
            Ok(handler.read_u64(reader)?)
        } else {
            Ok(handler.read_u32(reader)? as u64)
        }
    }

    /// Helper: Calculates the offset where next IFD pointer is located
    fn calculate_entries_end(&self, ifd_offset: u64, entry_count: u64) -> u64 {
        let entry_size = if self.is_big_tiff { 20 } else { 12 };
        let header_size = if self.is_big_tiff { 8 } else { 2 };
        ifd_offset + header_size + (entry_count * entry_size)
    }

    /// Opens a TIFF file with custom options
    ///
    /// # Arguments
    /// * `path` - Path to the TIFF file
    /// * `use_mmap` - Whether to use memory mapping (faster for large files)
    /// * `cache_size` - Number of tiles to cache (0 to disable caching)
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        use_mmap: bool,
        cache_size: usize,
    ) -> Result<Self> {
        let file = File::open(&path)?;
        let mut reader = BufferedReader::new(file);

        let byte_order = ByteOrder::detect(&mut reader)?;
        let handler = byte_order.handler();

        let magic = handler.read_u16(&mut reader)?;

        let is_big_tiff = match magic {
            TIFF_MAGIC => false,
            BIGTIFF_MAGIC => true,
            _ => return Err(Error::InvalidMagic(magic)),
        };

        if is_big_tiff {
            let offset_size = handler.read_u16(&mut reader)?;
            if offset_size != 8 {
                return Err(Error::InvalidFormat(
                    format!("Invalid BigTIFF offset size: {}", offset_size)
                ));
            }
            let _reserved = handler.read_u16(&mut reader)?;
        }

        let mmap = if use_mmap {
            let file_for_mmap = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file_for_mmap)? };

            #[cfg(unix)]
            unsafe {
                libc::madvise(
                    mmap.as_ptr() as *mut libc::c_void,
                    mmap.len(),
                    libc::MADV_SEQUENTIAL | libc::MADV_WILLNEED,
                );
            }

            Some(Arc::new(mmap))
        } else {
            None
        };

        let tile_reader = TileReader::new(reader, byte_order, mmap, cache_size);

        Ok(Self {
            tile_reader,
            byte_order,
            is_big_tiff,
            current_ifd_index: 0,
        })
    }

    /// Reads the TIFF file and returns the structure
    pub fn read(&mut self) -> Result<Tiff> {
        let mut tiff = Tiff::new(self.is_big_tiff);
        let mut next_ifd_offset = self.read_first_ifd_offset()?;
        let mut ifd_number = 0;

        while next_ifd_offset != 0 {
            if ifd_number > 1000 {
                return Err(Error::InvalidFormat("Too many IFDs".to_string()));
            }

            let ifd = self.read_ifd(ifd_number, next_ifd_offset)?;
            let entry_count = self.read_entry_count_at(next_ifd_offset)?;
            let entries_end = self.calculate_entries_end(next_ifd_offset, entry_count);

            next_ifd_offset = self.read_next_ifd_offset(entries_end)?;
            tiff.add_ifd(ifd);
            ifd_number += 1;
        }

        Ok(tiff)
    }

    /// Helper: Reads entry count at given offset
    fn read_entry_count_at(&mut self, offset: u64) -> Result<u64> {
        let handler = self.byte_order.handler();
        let reader = self.tile_reader.reader_mut();

        reader.seek(SeekFrom::Start(offset))?;

        if self.is_big_tiff {
            Ok(handler.read_u64(reader)?)
        } else {
            Ok(handler.read_u16(reader)? as u64)
        }
    }

    /// Reads a single IFD at the given offset
    fn read_ifd(&mut self, number: usize, offset: u64) -> Result<IFD> {
        let handler = self.byte_order.handler();
        let reader = self.tile_reader.reader_mut();

        reader.seek(SeekFrom::Start(offset))?;

        let entry_count = if self.is_big_tiff {
            handler.read_u64(reader)?
        } else {
            handler.read_u16(reader)? as u64
        };

        let mut ifd = IFD::new(number, offset);

        for _ in 0..entry_count {
            let tag = handler.read_u16(reader)?;
            let field_type = handler.read_u16(reader)?;

            let count = if self.is_big_tiff {
                handler.read_u64(reader)?
            } else {
                handler.read_u32(reader)? as u64
            };

            let value_offset = if self.is_big_tiff {
                handler.read_u64(reader)?
            } else {
                handler.read_u32(reader)? as u64
            };

            let entry = IFDEntry::new(tag, field_type, count, value_offset);
            ifd.add_entry(entry);
        }

        Ok(ifd)
    }

    /// Reads tag values as f64 array
    pub fn read_tag_doubles(&mut self, entry: &IFDEntry) -> Result<Vec<f64>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_doubles(entry)
    }

    /// Reads tag values as u16 array
    pub fn read_tag_u16s(&mut self, entry: &IFDEntry) -> Result<Vec<u16>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_u16s(entry)
    }

    /// Reads tag values as i16 array
    pub fn read_tag_i16s(&mut self, entry: &IFDEntry) -> Result<Vec<i16>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_i16s(entry)
    }

    /// Reads tag values as u32 array
    pub fn read_tag_u32s(&mut self, entry: &IFDEntry) -> Result<Vec<u32>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_u32s(entry)
    }

    /// Reads tag values as i32 array
    pub fn read_tag_i32s(&mut self, entry: &IFDEntry) -> Result<Vec<i32>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_i32s(entry)
    }

    /// Reads tag values as u64 array
    pub fn read_tag_u64s(&mut self, entry: &IFDEntry) -> Result<Vec<u64>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_u64s(entry)
    }

    /// Reads tag values as i64 array
    pub fn read_tag_i64s(&mut self, entry: &IFDEntry) -> Result<Vec<i64>> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_i64s(entry)
    }

    /// Reads ASCII string from tag
    pub fn read_tag_ascii(&mut self, entry: &IFDEntry) -> Result<String> {
        let handler = self.byte_order.handler();
        let mut tag_reader = TagReader::new(self.tile_reader.reader_mut(), &*handler, self.is_big_tiff);
        tag_reader.read_ascii(entry)
    }

    /// Reads a tile from the file with caching
    pub fn read_tile(&mut self, ifd: &IFD, tile_index: usize) -> Result<Vec<u8>> {
        self.tile_reader.set_current_ifd(self.current_ifd_index);
        self.tile_reader.read_tile(ifd, tile_index)
    }

    /// Helper: Reads pixel data and indices for any type
    fn read_pixel_data(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<(Vec<u8>, usize)> {
        PixelReader::validate_tiled_access(ifd)?;
        PixelReader::validate_pixel_bounds(ifd, x, y)?;

        let tile_index = PixelReader::calculate_tile_index(ifd, x, y)?;
        let tile_data = self.read_tile(ifd, tile_index)?;
        let pixel_index = PixelReader::calculate_pixel_index(ifd, x, y)?;

        Ok((tile_data, pixel_index))
    }

    /// Reads a pixel value at specific pixel coordinates (u8)
    pub fn read_pixel_value(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<u8> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_u8_from_tile(&tile_data, pixel_index)
    }

    /// Reads a u16 pixel value at specific coordinates
    pub fn read_pixel_u16(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<u16> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_u16_from_tile(&tile_data, pixel_index)
    }

    /// Reads an i16 pixel value at specific coordinates
    pub fn read_pixel_i16(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<i16> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_i16_from_tile(&tile_data, pixel_index)
    }

    /// Reads a u32 pixel value at specific coordinates
    pub fn read_pixel_u32(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<u32> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_u32_from_tile(&tile_data, pixel_index)
    }

    /// Reads an i32 pixel value at specific coordinates
    pub fn read_pixel_i32(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<i32> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_i32_from_tile(&tile_data, pixel_index)
    }

    /// Reads an f32 pixel value at specific coordinates
    pub fn read_pixel_f32(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<f32> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_f32_from_tile(&tile_data, pixel_index)
    }

    /// Reads an f64 pixel value at specific coordinates
    pub fn read_pixel_f64(&mut self, ifd: &IFD, x: u64, y: u64) -> Result<f64> {
        let (tile_data, pixel_index) = self.read_pixel_data(ifd, x, y)?;
        PixelReader::read_f64_from_tile(&tile_data, pixel_index)
    }

    /// Reads multiple tiles in parallel
    pub fn read_tiles_parallel(&mut self, ifd: &IFD, tile_indices: &[usize]) -> Result<Vec<Vec<u8>>> {
        self.tile_reader.set_current_ifd(self.current_ifd_index);

        ParallelReader::read_tiles_parallel(
            ifd,
            tile_indices,
            self.tile_reader.cache(),
            self.current_ifd_index,
            self.tile_reader.mmap(),
            self.tile_reader.byte_order(),
        )
    }

    /// Reads a pixel value at geographic coordinates
    pub fn read_pixel_at_coord(&mut self, ifd: &IFD, geo_x: f64, geo_y: f64) -> Result<u8> {
        use super::geotiff::GeoInfo;

        let geo_info = GeoInfo::from_ifd(ifd, self)?
            .ok_or_else(|| Error::InvalidFormat("Not a GeoTIFF".to_string()))?;

        let transform = geo_info.affine_transform()
            .ok_or_else(|| Error::InvalidFormat("Missing geotransform".to_string()))?;

        // Convert geographic to pixel coordinates using inverse transform
        let det = transform[1] * transform[5] - transform[2] * transform[4];
        if det.abs() < 1e-10 {
            return Err(Error::InvalidFormat("Singular transform matrix".to_string()));
        }

        let pixel_x = ((geo_x - transform[0]) * transform[5]
                      - (geo_y - transform[3]) * transform[2]) / det;
        let pixel_y = ((geo_y - transform[3]) * transform[1]
                      - (geo_x - transform[0]) * transform[4]) / det;

        if pixel_x < 0.0 || pixel_y < 0.0 {
            return Err(Error::OutOfBounds(format!(
                "Coordinate ({}, {}) maps to negative pixel coordinates",
                geo_x, geo_y
            )));
        }

        self.read_pixel_value(ifd, pixel_x as u64, pixel_y as u64)
    }

    /// Reads multiple pixel values in parallel by batching tile loads
    ///
    /// This method groups pixels by their tiles, loads tiles in parallel,
    /// then extracts all pixel values. Much faster than individual reads.
    /// Tiles are loaded directly without caching to avoid cache eviction issues.
    ///
    /// # Arguments
    /// * `ifd` - The IFD to read from
    /// * `coords` - Vec of (x, y) pixel coordinates
    ///
    /// # Returns
    /// Vec of pixel values in the same order as input coordinates
    pub fn read_pixels_batch(&mut self, ifd: &IFD, coords: &[(u64, u64)]) -> Result<Vec<u8>> {
        use std::collections::HashMap;

        let mut tile_pixels: HashMap<usize, Vec<(usize, u64, u64)>> = HashMap::new();

        for (result_idx, &(x, y)) in coords.iter().enumerate() {
            PixelReader::validate_tiled_access(ifd)?;
            PixelReader::validate_pixel_bounds(ifd, x, y)?;
            let tile_index = PixelReader::calculate_tile_index(ifd, x, y)?;

            tile_pixels.entry(tile_index)
                .or_insert_with(Vec::new)
                .push((result_idx, x, y));
        }

        let tile_indices: Vec<usize> = tile_pixels.keys().copied().collect();

        let tiles = ParallelReader::read_tiles_parallel_direct(
            ifd,
            &tile_indices,
            self.tile_reader.mmap(),
            self.tile_reader.byte_order(),
        )?;

        let tile_map: HashMap<usize, &Vec<u8>> = tile_indices.iter()
            .zip(tiles.iter())
            .map(|(&idx, data)| (idx, data))
            .collect();

        let mut results = vec![0u8; coords.len()];

        for (tile_index, pixels) in tile_pixels.iter() {
            let tile_data = tile_map.get(tile_index)
                .ok_or_else(|| Error::InvalidFormat("Missing tile data".to_string()))?;

            for &(result_idx, x, y) in pixels {
                let pixel_index = PixelReader::calculate_pixel_index(ifd, x, y)?;
                results[result_idx] = PixelReader::read_u8_from_tile(tile_data, pixel_index)?;
            }
        }

        Ok(results)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_minimal_tiff() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();

        file.write_all(b"II").unwrap();
        file.write_all(&42u16.to_le_bytes()).unwrap();
        file.write_all(&8u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&256u16.to_le_bytes()).unwrap();
        file.write_all(&4u16.to_le_bytes()).unwrap();
        file.write_all(&1u32.to_le_bytes()).unwrap();
        file.write_all(&1024u32.to_le_bytes()).unwrap();
        file.write_all(&0u32.to_le_bytes()).unwrap();

        file.flush().unwrap();
        file
    }

    #[test]
    fn test_open_tiff() {
        let file = create_minimal_tiff();
        let reader = TiffReader::open(file.path());
        assert!(reader.is_ok());
        let reader = reader.unwrap();
        assert!(!reader.is_big_tiff);
    }

    #[test]
    fn test_read_tiff() {
        let file = create_minimal_tiff();
        let mut reader = TiffReader::open(file.path()).unwrap();
        let tiff = reader.read().unwrap();
        assert!(!tiff.is_big_tiff);
        assert_eq!(tiff.ifd_count(), 1);
    }
}
