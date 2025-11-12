//! GeoTIFF specific functionality

use crate::error::Result;
use crate::projection::{Coordinate, Transformer};
use super::ifd::IFD;
use super::tags;
use super::reader::TiffReader;

/// GeoTIFF information extracted from an IFD
#[derive(Debug)]
pub struct GeoInfo {
    /// Model pixel scale (ScaleX, ScaleY, ScaleZ)
    pub pixel_scale: Option<(f64, f64, f64)>,
    /// Model tiepoint (pixel coord -> geo coord mapping)
    pub tiepoints: Vec<TiePoint>,
    /// GeoTransform (if available)
    pub transform: Option<[f64; 16]>,
    /// EPSG code if detected
    pub epsg_code: Option<u16>,
    /// CRS name
    pub crs_name: Option<String>,
}

/// Represents a GeoTIFF tiepoint
#[derive(Debug)]
pub struct TiePoint {
    pub pixel_x: f64,
    pub pixel_y: f64,
    pub pixel_z: f64,
    pub geo_x: f64,
    pub geo_y: f64,
    pub geo_z: f64,
}

/// GeoKey constants
mod geo_keys {
    pub const GEOGRAPHIC_TYPE: u16 = 2048;
    pub const PROJECTED_CS_TYPE: u16 = 3072;
}

impl GeoInfo {
    /// Extracts GeoTIFF information from an IFD
    pub fn from_ifd(ifd: &IFD, reader: &mut TiffReader) -> Result<Option<Self>> {
        let has_geo_tags = ifd.get_entry(tags::MODEL_PIXEL_SCALE).is_some()
            || ifd.get_entry(tags::MODEL_TIEPOINT).is_some()
            || ifd.get_entry(tags::GEO_KEY_DIRECTORY).is_some();

        if !has_geo_tags {
            return Ok(None);
        }

        let mut geo_info = GeoInfo {
            pixel_scale: None,
            tiepoints: Vec::new(),
            transform: None,
            epsg_code: None,
            crs_name: None,
        };

        if let Some(entry) = ifd.get_entry(tags::MODEL_PIXEL_SCALE) {
            let values = reader.read_tag_doubles(entry)?;
            if values.len() >= 3 {
                geo_info.pixel_scale = Some((values[0], values[1], values[2]));
            }
        }

        if let Some(entry) = ifd.get_entry(tags::MODEL_TIEPOINT) {
            let values = reader.read_tag_doubles(entry)?;
            for chunk in values.chunks(6) {
                if chunk.len() == 6 {
                    geo_info.tiepoints.push(TiePoint {
                        pixel_x: chunk[0],
                        pixel_y: chunk[1],
                        pixel_z: chunk[2],
                        geo_x: chunk[3],
                        geo_y: chunk[4],
                        geo_z: chunk[5],
                    });
                }
            }
        }

        if let Some(entry) = ifd.get_entry(tags::GEO_KEY_DIRECTORY) {
            let keys = reader.read_tag_u16s(entry)?;

            if keys.len() >= 4 {
                let num_keys = keys[3] as usize;

                for i in 0..num_keys {
                    let offset = 4 + (i * 4);
                    if offset + 3 < keys.len() {
                        let key_id = keys[offset];
                        let _location = keys[offset + 1];
                        let _count = keys[offset + 2];
                        let value_offset = keys[offset + 3];

                        match key_id {
                            geo_keys::GEOGRAPHIC_TYPE | geo_keys::PROJECTED_CS_TYPE => {
                                geo_info.epsg_code = Some(value_offset);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let Some(entry) = ifd.get_entry(tags::GEO_ASCII_PARAMS) {
            let ascii = reader.read_tag_ascii(entry)?;
            if !ascii.is_empty() {
                geo_info.crs_name = Some(ascii);
            }
        }

        Ok(Some(geo_info))
    }

    /// Computes the affine transform from pixel to geo coordinates
    ///
    /// Returns [a, b, c, d, e, f] where:
    /// geo_x = a + b * pixel_x + c * pixel_y
    /// geo_y = d + e * pixel_x + f * pixel_y
    pub fn affine_transform(&self) -> Option<[f64; 6]> {
        if let (Some((scale_x, scale_y, _)), Some(tp)) = (&self.pixel_scale, self.tiepoints.first()) {
            Some([
                tp.geo_x - scale_x * tp.pixel_x,
                *scale_x,
                0.0,
                tp.geo_y + scale_y * tp.pixel_y,
                0.0,
                -scale_y,
            ])
        } else {
            None
        }
    }

    /// Computes the bounding box in geo coordinates
    ///
    /// Returns (min_x, min_y, max_x, max_y)
    pub fn bounding_box(&self, width: u64, height: u64) -> Option<(f64, f64, f64, f64)> {
        let transform = self.affine_transform()?;

        let min_x = transform[0];
        let max_y = transform[3];
        let max_x = min_x + (transform[1] * width as f64);
        let min_y = max_y + (transform[5] * height as f64);

        Some((min_x, min_y, max_x, max_y))
    }

    /// Converts pixel coordinates to geographic coordinates
    pub fn pixel_to_geo(&self, pixel_x: u64, pixel_y: u64) -> Option<Coordinate> {
        let transform = self.affine_transform()?;

        let geo_x = transform[0] + transform[1] * pixel_x as f64 + transform[2] * pixel_y as f64;
        let geo_y = transform[3] + transform[4] * pixel_x as f64 + transform[5] * pixel_y as f64;

        Some(Coordinate::new(geo_x, geo_y))
    }

    /// Converts geographic coordinates to pixel coordinates
    pub fn geo_to_pixel(&self, geo_coord: Coordinate) -> Option<(f64, f64)> {
        let transform = self.affine_transform()?;

        let det = transform[1] * transform[5] - transform[2] * transform[4];
        if det.abs() < 1e-10 {
            return None;
        }

        let dx = geo_coord.x - transform[0];
        let dy = geo_coord.y - transform[3];

        let pixel_x = (transform[5] * dx - transform[2] * dy) / det;
        let pixel_y = (-transform[4] * dx + transform[1] * dy) / det;

        Some((pixel_x, pixel_y))
    }

    /// Transforms a coordinate to a different CRS
    pub fn transform_coordinate(&self, coord: Coordinate, target_epsg: u16) -> Result<Coordinate> {
        let source_epsg = self.epsg_code
            .ok_or_else(|| crate::error::Error::Projection("No EPSG code available".to_string()))?;

        let transformer = Transformer::new(source_epsg, target_epsg)?;
        transformer.transform(coord)
    }

    /// Converts pixel coordinates to a different CRS
    pub fn transform_pixel_to_crs(&self, pixel_x: u64, pixel_y: u64, target_epsg: u16) -> Result<Coordinate> {
        let geo_coord = self.pixel_to_geo(pixel_x, pixel_y)
            .ok_or_else(|| crate::error::Error::Projection("Failed to convert pixel to geo".to_string()))?;

        self.transform_coordinate(geo_coord, target_epsg)
    }

    /// Converts coordinates from a different CRS to pixel coordinates
    pub fn transform_crs_to_pixel(&self, coord: Coordinate, source_epsg: u16) -> Result<(f64, f64)> {
        let target_epsg = self.epsg_code
            .ok_or_else(|| crate::error::Error::Projection("No EPSG code available".to_string()))?;

        let transformer = Transformer::new(source_epsg, target_epsg)?;
        let geo_coord = transformer.transform(coord)?;

        self.geo_to_pixel(geo_coord)
            .ok_or_else(|| crate::error::Error::Projection("Failed to convert geo to pixel".to_string()))
    }

    /// Batch transform coordinates from source CRS to pixel coordinates.
    /// Reuses transformer object for performance.
    pub fn transform_crs_to_pixel_batch(
        &self,
        coords: &[Coordinate],
        source_epsg: u16,
    ) -> Result<Vec<(f64, f64)>> {
        let target_epsg = self.epsg_code
            .ok_or_else(|| crate::error::Error::Projection("No EPSG code available".to_string()))?;

        let transformer = Transformer::new(source_epsg, target_epsg)?;

        coords.iter()
            .map(|&coord| {
                let geo_coord = transformer.transform(coord)?;
                self.geo_to_pixel(geo_coord)
                    .ok_or_else(|| crate::error::Error::Projection("Failed to convert geo to pixel".to_string()))
            })
            .collect()
    }
}

impl std::fmt::Display for GeoInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\nGeoTIFF Information:")?;

        if let Some(epsg) = self.epsg_code {
            writeln!(f, "  EPSG Code: {}", epsg)?;
        }

        if let Some(ref name) = self.crs_name {
            writeln!(f, "  CRS Name: {}", name)?;
        }

        if let Some((sx, sy, _sz)) = self.pixel_scale {
            writeln!(f, "  Pixel Size: {} x {}", sx, sy)?;
        }

        if let Some(tp) = self.tiepoints.first() {
            writeln!(f, "  Origin (geo): ({}, {})", tp.geo_x, tp.geo_y)?;
        }

        Ok(())
    }
}
