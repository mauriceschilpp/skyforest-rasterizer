//! Image File Directory (IFD) structures

use std::collections::HashMap;
use crate::types::{Dimensions, DataType};
use super::tags;

/// Represents an Image File Directory entry
#[derive(Debug, Clone)]
pub struct IFDEntry {
    /// TIFF tag identifier
    pub tag: u16,
    /// Field type
    pub field_type: u16,
    /// Number of values
    pub count: u64,
    /// Value or offset to value
    pub value_offset: u64,
}

impl IFDEntry {
    /// Creates a new IFD entry
    pub fn new(tag: u16, field_type: u16, count: u64, value_offset: u64) -> Self {
        Self {
            tag,
            field_type,
            count,
            value_offset,
        }
    }

    /// Returns the size in bytes of this field type
    pub fn field_type_size(&self) -> usize {
        use super::tags::field_types::*;
        match self.field_type {
            BYTE | ASCII | SBYTE | UNDEFINED => 1,
            SHORT | SSHORT => 2,
            LONG | SLONG | FLOAT => 4,
            RATIONAL | SRATIONAL | DOUBLE | LONG8 | SLONG8 | IFD8 => 8,
            _ => 1,
        }
    }

    /// Returns whether the value is stored inline (in value_offset field)
    pub fn is_inline(&self, is_big_tiff: bool) -> bool {
        let total_size = self.field_type_size() * self.count as usize;
        let inline_size = if is_big_tiff { 8 } else { 4 };
        total_size <= inline_size
    }
}

/// Represents an Image File Directory
#[derive(Debug, Clone)]
pub struct IFD {
    /// IFD number (0-based)
    pub number: usize,
    /// Offset to this IFD in file
    pub offset: u64,
    /// Entries in this IFD
    pub entries: Vec<IFDEntry>,
    /// Tag map for quick lookup
    tag_map: HashMap<u16, usize>,
}

impl IFD {
    /// Creates a new IFD
    pub fn new(number: usize, offset: u64) -> Self {
        Self {
            number,
            offset,
            entries: Vec::new(),
            tag_map: HashMap::new(),
        }
    }

    /// Adds an entry to this IFD
    pub fn add_entry(&mut self, entry: IFDEntry) {
        let index = self.entries.len();
        self.tag_map.insert(entry.tag, index);
        self.entries.push(entry);
    }

    /// Gets an entry by tag
    pub fn get_entry(&self, tag: u16) -> Option<&IFDEntry> {
        self.tag_map.get(&tag).and_then(|&idx| self.entries.get(idx))
    }

    /// Gets the value of a tag as u64 (for inline values)
    pub fn get_tag_value(&self, tag: u16) -> Option<u64> {
        self.get_entry(tag).map(|e| e.value_offset)
    }

    /// Returns image dimensions if available
    pub fn dimensions(&self) -> Option<Dimensions> {
        let width = self.get_tag_value(tags::IMAGE_WIDTH)?;
        let height = self.get_tag_value(tags::IMAGE_LENGTH)?;
        Some(Dimensions::new(width, height))
    }

    /// Returns compression type
    pub fn compression(&self) -> Option<u64> {
        self.get_tag_value(tags::COMPRESSION)
    }

    /// Returns samples per pixel
    pub fn samples_per_pixel(&self) -> u64 {
        self.get_tag_value(tags::SAMPLES_PER_PIXEL).unwrap_or(1)
    }

    /// Returns bits per sample
    pub fn bits_per_sample(&self) -> Option<u64> {
        self.get_tag_value(tags::BITS_PER_SAMPLE)
    }

    /// Returns sample format (1=unsigned, 2=signed, 3=float)
    pub fn sample_format(&self) -> u64 {
        self.get_tag_value(tags::SAMPLE_FORMAT).unwrap_or(1) // Default is unsigned
    }

    /// Determines the pixel data type based on TIFF tags
    pub fn data_type(&self) -> Option<DataType> {
        let bits = self.bits_per_sample()?;
        let format = self.sample_format();

        match (format, bits) {
            (1, 8) => Some(DataType::U8),
            (1, 16) => Some(DataType::U16),
            (1, 32) => Some(DataType::U32),
            (2, 8) => Some(DataType::I8),
            (2, 16) => Some(DataType::I16),
            (2, 32) => Some(DataType::I32),
            (3, 32) => Some(DataType::F32),
            (3, 64) => Some(DataType::F64),
            _ => None,
        }
    }

    /// Returns whether this IFD represents a tiled image
    pub fn is_tiled(&self) -> bool {
        self.get_entry(tags::TILE_WIDTH).is_some()
    }

    /// Returns tile dimensions if tiled
    pub fn tile_dimensions(&self) -> Option<Dimensions> {
        let width = self.get_tag_value(tags::TILE_WIDTH)?;
        let height = self.get_tag_value(tags::TILE_LENGTH)?;
        Some(Dimensions::new(width, height))
    }

    /// Returns number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns all GeoTIFF related tags
    pub fn geotiff_tags(&self) -> Vec<&IFDEntry> {
        self.entries.iter()
            .filter(|e| {
                matches!(e.tag,
                    tags::MODEL_PIXEL_SCALE |
                    tags::MODEL_TIEPOINT |
                    tags::MODEL_TRANSFORMATION |
                    tags::GEO_KEY_DIRECTORY |
                    tags::GEO_DOUBLE_PARAMS |
                    tags::GEO_ASCII_PARAMS
                )
            })
            .collect()
    }

    /// Checks if this IFD has GeoTIFF tags
    pub fn is_geotiff(&self) -> bool {
        !self.geotiff_tags().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ifd_entry_creation() {
        let entry = IFDEntry::new(256, 4, 1, 1024);
        assert_eq!(entry.tag, 256);
        assert_eq!(entry.field_type, 4);
        assert_eq!(entry.count, 1);
        assert_eq!(entry.value_offset, 1024);
    }

    #[test]
    fn test_field_type_size() {
        let entry = IFDEntry::new(256, tags::field_types::BYTE, 1, 0);
        assert_eq!(entry.field_type_size(), 1);

        let entry = IFDEntry::new(256, tags::field_types::SHORT, 1, 0);
        assert_eq!(entry.field_type_size(), 2);

        let entry = IFDEntry::new(256, tags::field_types::LONG, 1, 0);
        assert_eq!(entry.field_type_size(), 4);

        let entry = IFDEntry::new(256, tags::field_types::DOUBLE, 1, 0);
        assert_eq!(entry.field_type_size(), 8);
    }

    #[test]
    fn test_is_inline() {
        let entry = IFDEntry::new(256, tags::field_types::SHORT, 1, 0);
        assert!(entry.is_inline(false));

        let entry = IFDEntry::new(256, tags::field_types::LONG, 2, 0);
        assert!(!entry.is_inline(false));
        assert!(entry.is_inline(true));
    }

    #[test]
    fn test_ifd_creation() {
        let ifd = IFD::new(0, 1000);
        assert_eq!(ifd.number, 0);
        assert_eq!(ifd.offset, 1000);
        assert_eq!(ifd.entry_count(), 0);
    }

    #[test]
    fn test_add_and_get_entry() {
        let mut ifd = IFD::new(0, 1000);
        let entry = IFDEntry::new(tags::IMAGE_WIDTH, tags::field_types::LONG, 1, 1024);
        ifd.add_entry(entry);

        assert_eq!(ifd.entry_count(), 1);
        assert!(ifd.get_entry(tags::IMAGE_WIDTH).is_some());
        assert_eq!(ifd.get_tag_value(tags::IMAGE_WIDTH), Some(1024));
    }

    #[test]
    fn test_dimensions() {
        let mut ifd = IFD::new(0, 1000);
        ifd.add_entry(IFDEntry::new(tags::IMAGE_WIDTH, tags::field_types::LONG, 1, 1024));
        ifd.add_entry(IFDEntry::new(tags::IMAGE_LENGTH, tags::field_types::LONG, 1, 768));

        let dims = ifd.dimensions().unwrap();
        assert_eq!(dims.width, 1024);
        assert_eq!(dims.height, 768);
    }

    #[test]
    fn test_samples_per_pixel() {
        let mut ifd = IFD::new(0, 1000);
        assert_eq!(ifd.samples_per_pixel(), 1);

        ifd.add_entry(IFDEntry::new(tags::SAMPLES_PER_PIXEL, tags::field_types::SHORT, 1, 3));
        assert_eq!(ifd.samples_per_pixel(), 3);
    }

    #[test]
    fn test_is_tiled() {
        let mut ifd = IFD::new(0, 1000);
        assert!(!ifd.is_tiled());

        ifd.add_entry(IFDEntry::new(tags::TILE_WIDTH, tags::field_types::LONG, 1, 256));
        assert!(ifd.is_tiled());
    }
}
