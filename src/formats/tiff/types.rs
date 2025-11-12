//! TIFF data structures

use super::ifd::IFD;
use std::fmt;

/// Represents a TIFF or BigTIFF file
#[derive(Debug)]
pub struct Tiff {
    /// Whether this is BigTIFF format
    pub is_big_tiff: bool,
    /// Image File Directories
    pub ifds: Vec<IFD>,
}

impl Tiff {
    /// Creates a new TIFF structure
    pub fn new(is_big_tiff: bool) -> Self {
        Self {
            is_big_tiff,
            ifds: Vec::new(),
        }
    }

    /// Adds an IFD to this TIFF
    pub fn add_ifd(&mut self, ifd: IFD) {
        self.ifds.push(ifd);
    }

    /// Returns the main (first) IFD
    pub fn main_ifd(&self) -> Option<&IFD> {
        self.ifds.first()
    }

    /// Returns the number of IFDs
    pub fn ifd_count(&self) -> usize {
        self.ifds.len()
    }

    /// Returns all IFDs
    pub fn all_ifds(&self) -> &[IFD] {
        &self.ifds
    }
}

impl fmt::Display for Tiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "TIFF File Information:")?;
        writeln!(f, "  Format: {}", if self.is_big_tiff { "BigTIFF" } else { "TIFF" })?;
        writeln!(f, "  Number of IFDs: {}", self.ifds.len())?;

        if let Some(ifd) = self.main_ifd() {
            writeln!(f, "\nMain Image (IFD 0):")?;
            if let Some(dims) = ifd.dimensions() {
                writeln!(f, "  Dimensions: {} x {}", dims.width, dims.height)?;
            }
            writeln!(f, "  Samples per pixel: {}", ifd.samples_per_pixel())?;
            if let Some(bits) = ifd.bits_per_sample() {
                writeln!(f, "  Bits per sample: {}", bits)?;
            }
            if let Some(compression) = ifd.compression() {
                writeln!(f, "  Compression: {}", compression)?;
            }
            writeln!(f, "  Tiled: {}", if ifd.is_tiled() { "Yes" } else { "No" })?;
            if let Some(tile_dims) = ifd.tile_dimensions() {
                writeln!(f, "  Tile size: {} x {}", tile_dims.width, tile_dims.height)?;
            }
            writeln!(f, "  GeoTIFF: {}", if ifd.is_geotiff() { "Yes" } else { "No" })?;

            if ifd.is_geotiff() {
                writeln!(f, "\nGeoTIFF Tags Found:")?;
                for tag in ifd.geotiff_tags() {
                    writeln!(f, "  Tag {}: {} ({} values)",
                        tag.tag,
                        super::tags::tag_name(tag.tag),
                        tag.count
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::tiff::ifd::IFDEntry;
    use crate::formats::tiff::tags;

    #[test]
    fn test_tiff_creation() {
        let tiff = Tiff::new(false);
        assert!(!tiff.is_big_tiff);
        assert_eq!(tiff.ifd_count(), 0);
    }

    #[test]
    fn test_bigtiff_creation() {
        let tiff = Tiff::new(true);
        assert!(tiff.is_big_tiff);
    }

    #[test]
    fn test_add_ifd() {
        let mut tiff = Tiff::new(false);
        let ifd = IFD::new(0, 1000);
        tiff.add_ifd(ifd);

        assert_eq!(tiff.ifd_count(), 1);
        assert!(tiff.main_ifd().is_some());
    }

    #[test]
    fn test_main_ifd() {
        let mut tiff = Tiff::new(false);
        assert!(tiff.main_ifd().is_none());

        let mut ifd = IFD::new(0, 1000);
        ifd.add_entry(IFDEntry::new(tags::IMAGE_WIDTH, tags::field_types::LONG, 1, 1024));
        tiff.add_ifd(ifd);

        let main = tiff.main_ifd().unwrap();
        assert_eq!(main.number, 0);
        assert_eq!(main.get_tag_value(tags::IMAGE_WIDTH), Some(1024));
    }

    #[test]
    fn test_display() {
        let mut tiff = Tiff::new(true);
        let mut ifd = IFD::new(0, 8);
        ifd.add_entry(IFDEntry::new(tags::IMAGE_WIDTH, tags::field_types::LONG, 1, 1024));
        ifd.add_entry(IFDEntry::new(tags::IMAGE_LENGTH, tags::field_types::LONG, 1, 768));
        ifd.add_entry(IFDEntry::new(tags::SAMPLES_PER_PIXEL, tags::field_types::SHORT, 1, 3));
        tiff.add_ifd(ifd);

        let output = format!("{}", tiff);
        assert!(output.contains("BigTIFF"));
        assert!(output.contains("1024 x 768"));
        assert!(output.contains("Samples per pixel: 3"));
    }
}
