//! Pixel value reading operations

use crate::error::{Error, Result};
use crate::formats::tiff::IFD;

/// Handles pixel value reading and coordinate calculations
pub struct PixelReader;

impl PixelReader {
    /// Validates that the IFD supports tiled access
    pub fn validate_tiled_access(ifd: &IFD) -> Result<()> {
        if !ifd.is_tiled() {
            return Err(Error::Unsupported("Only tiled images supported".to_string()));
        }
        Ok(())
    }

    /// Validates that pixel coordinates are within bounds
    pub fn validate_pixel_bounds(ifd: &IFD, x: u64, y: u64) -> Result<()> {
        let dims = ifd.dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing dimensions".to_string()))?;

        if x >= dims.width || y >= dims.height {
            return Err(Error::OutOfBounds(format!(
                "Pixel ({}, {}) outside image bounds ({}, {})",
                x, y, dims.width, dims.height
            )));
        }

        Ok(())
    }

    /// Calculates which tile contains a pixel
    pub fn calculate_tile_index(ifd: &IFD, x: u64, y: u64) -> Result<usize> {
        let dims = ifd.dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing dimensions".to_string()))?;

        let tile_dims = ifd.tile_dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing tile dimensions".to_string()))?;

        let tile_x = x / tile_dims.width;
        let tile_y = y / tile_dims.height;
        let tiles_across = dims.width.div_ceil(tile_dims.width);
        Ok((tile_y * tiles_across + tile_x) as usize)
    }

    /// Calculates pixel index within a tile
    pub fn calculate_pixel_index(ifd: &IFD, x: u64, y: u64) -> Result<usize> {
        let tile_dims = ifd.tile_dimensions()
            .ok_or_else(|| Error::InvalidFormat("Missing tile dimensions".to_string()))?;

        let pixel_x = (x % tile_dims.width) as usize;
        let pixel_y = (y % tile_dims.height) as usize;
        Ok(pixel_y * tile_dims.width as usize + pixel_x)
    }

    /// Reads a u8 pixel value from tile data
    pub fn read_u8_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<u8> {
        tile_data.get(pixel_index)
            .copied()
            .ok_or_else(|| Error::OutOfBounds(format!(
                "Pixel index {} exceeds tile data length {}",
                pixel_index, tile_data.len()
            )))
    }

    /// Reads N bytes for a pixel value from tile data
    pub fn read_bytes_from_tile<const N: usize>(
        tile_data: &[u8],
        pixel_index: usize,
    ) -> Result<[u8; N]> {
        let byte_offset = pixel_index * N;

        if byte_offset + N > tile_data.len() {
            return Err(Error::OutOfBounds(format!(
                "Pixel offset {} exceeds tile data length {}",
                byte_offset, tile_data.len()
            )));
        }

        let mut bytes = [0u8; N];
        bytes.copy_from_slice(&tile_data[byte_offset..byte_offset + N]);
        Ok(bytes)
    }

    /// Reads a u16 pixel value from tile data
    pub fn read_u16_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<u16> {
        let bytes = Self::read_bytes_from_tile::<2>(tile_data, pixel_index)?;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Reads an i16 pixel value from tile data
    pub fn read_i16_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<i16> {
        let bytes = Self::read_bytes_from_tile::<2>(tile_data, pixel_index)?;
        Ok(i16::from_le_bytes(bytes))
    }

    /// Reads a u32 pixel value from tile data
    pub fn read_u32_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<u32> {
        let bytes = Self::read_bytes_from_tile::<4>(tile_data, pixel_index)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Reads an i32 pixel value from tile data
    pub fn read_i32_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<i32> {
        let bytes = Self::read_bytes_from_tile::<4>(tile_data, pixel_index)?;
        Ok(i32::from_le_bytes(bytes))
    }

    /// Reads an f32 pixel value from tile data
    pub fn read_f32_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<f32> {
        let bytes = Self::read_bytes_from_tile::<4>(tile_data, pixel_index)?;
        Ok(f32::from_le_bytes(bytes))
    }

    /// Reads an f64 pixel value from tile data
    pub fn read_f64_from_tile(tile_data: &[u8], pixel_index: usize) -> Result<f64> {
        let bytes = Self::read_bytes_from_tile::<8>(tile_data, pixel_index)?;
        Ok(f64::from_le_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::tiff::{IFD, IFDEntry, tags};
    use crate::formats::tiff::tags::field_types;

    fn create_test_ifd(width: u64, height: u64, tile_width: u64, tile_height: u64) -> IFD {
        let mut ifd = IFD::new(0, 0);
        ifd.add_entry(IFDEntry::new(tags::IMAGE_WIDTH, field_types::LONG, 1, width));
        ifd.add_entry(IFDEntry::new(tags::IMAGE_LENGTH, field_types::LONG, 1, height));
        ifd.add_entry(IFDEntry::new(tags::TILE_WIDTH, field_types::LONG, 1, tile_width));
        ifd.add_entry(IFDEntry::new(tags::TILE_LENGTH, field_types::LONG, 1, tile_height));
        ifd
    }

    #[test]
    fn test_validate_tiled_access() {
        let ifd = create_test_ifd(512, 512, 256, 256);
        assert!(PixelReader::validate_tiled_access(&ifd).is_ok());
    }

    #[test]
    fn test_validate_pixel_bounds_valid() {
        let ifd = create_test_ifd(512, 512, 256, 256);
        assert!(PixelReader::validate_pixel_bounds(&ifd, 0, 0).is_ok());
        assert!(PixelReader::validate_pixel_bounds(&ifd, 511, 511).is_ok());
        assert!(PixelReader::validate_pixel_bounds(&ifd, 256, 256).is_ok());
    }

    #[test]
    fn test_validate_pixel_bounds_invalid() {
        let ifd = create_test_ifd(512, 512, 256, 256);
        assert!(PixelReader::validate_pixel_bounds(&ifd, 512, 0).is_err());
        assert!(PixelReader::validate_pixel_bounds(&ifd, 0, 512).is_err());
        assert!(PixelReader::validate_pixel_bounds(&ifd, 1000, 1000).is_err());
    }

    #[test]
    fn test_calculate_tile_index() {
        let ifd = create_test_ifd(512, 512, 256, 256);

        assert_eq!(PixelReader::calculate_tile_index(&ifd, 0, 0).unwrap(), 0);
        assert_eq!(PixelReader::calculate_tile_index(&ifd, 255, 255).unwrap(), 0);
        assert_eq!(PixelReader::calculate_tile_index(&ifd, 256, 0).unwrap(), 1);
        assert_eq!(PixelReader::calculate_tile_index(&ifd, 0, 256).unwrap(), 2);
        assert_eq!(PixelReader::calculate_tile_index(&ifd, 256, 256).unwrap(), 3);
    }

    #[test]
    fn test_calculate_pixel_index() {
        let ifd = create_test_ifd(512, 512, 256, 256);

        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 0, 0).unwrap(), 0);
        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 1, 0).unwrap(), 1);
        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 0, 1).unwrap(), 256);
        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 10, 5).unwrap(), 5 * 256 + 10);

        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 256, 0).unwrap(), 0);
        assert_eq!(PixelReader::calculate_pixel_index(&ifd, 257, 0).unwrap(), 1);
    }

    #[test]
    fn test_read_u8_from_tile() {
        let tile_data = vec![10, 20, 30, 40];
        assert_eq!(PixelReader::read_u8_from_tile(&tile_data, 0).unwrap(), 10);
        assert_eq!(PixelReader::read_u8_from_tile(&tile_data, 2).unwrap(), 30);
        assert!(PixelReader::read_u8_from_tile(&tile_data, 10).is_err());
    }

    #[test]
    fn test_read_u16_from_tile() {
        let tile_data = vec![0x01, 0x00, 0xFF, 0x00, 0x34, 0x12];
        assert_eq!(PixelReader::read_u16_from_tile(&tile_data, 0).unwrap(), 1);
        assert_eq!(PixelReader::read_u16_from_tile(&tile_data, 1).unwrap(), 255);
        assert_eq!(PixelReader::read_u16_from_tile(&tile_data, 2).unwrap(), 0x1234);
    }

    #[test]
    fn test_read_i16_from_tile() {
        let tile_data = vec![0xFF, 0xFF, 0x00, 0x00];
        assert_eq!(PixelReader::read_i16_from_tile(&tile_data, 0).unwrap(), -1);
        assert_eq!(PixelReader::read_i16_from_tile(&tile_data, 1).unwrap(), 0);
    }

    #[test]
    fn test_read_f32_from_tile() {
        let mut tile_data = vec![];
        tile_data.extend_from_slice(&1.5f32.to_le_bytes());
        tile_data.extend_from_slice(&(-2.5f32).to_le_bytes());

        assert_eq!(PixelReader::read_f32_from_tile(&tile_data, 0).unwrap(), 1.5);
        assert_eq!(PixelReader::read_f32_from_tile(&tile_data, 1).unwrap(), -2.5);
    }

    #[test]
    fn test_read_f64_from_tile() {
        let mut tile_data = vec![];
        tile_data.extend_from_slice(&3.14159f64.to_le_bytes());

        assert_eq!(PixelReader::read_f64_from_tile(&tile_data, 0).unwrap(), 3.14159);
    }

    #[test]
    fn test_read_bytes_out_of_bounds() {
        let tile_data = vec![1, 2, 3, 4];
        assert!(PixelReader::read_bytes_from_tile::<4>(&tile_data, 1).is_err());
        assert!(PixelReader::read_bytes_from_tile::<8>(&tile_data, 0).is_err());
    }
}
