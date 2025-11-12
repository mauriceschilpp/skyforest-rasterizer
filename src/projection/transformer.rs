use crate::error::{Error, Result};
use crate::projection::coordinate::Coordinate;
use proj::Proj;

/// Transforms coordinates between different coordinate reference systems
pub struct Transformer {
    proj: Proj,
    from_epsg: u16,
    to_epsg: u16,
}

impl Transformer {
    /// Creates a new transformer from source to target CRS using EPSG codes
    pub fn new(from_epsg: u16, to_epsg: u16) -> Result<Self> {
        let from = format!("EPSG:{}", from_epsg);
        let to = format!("EPSG:{}", to_epsg);

        let proj = Proj::new_known_crs(&from, &to, None)
            .map_err(|e| Error::Projection(format!("Failed to create projection: {}", e)))?;

        Ok(Self {
            proj,
            from_epsg,
            to_epsg,
        })
    }

    /// Creates a transformer from a custom PROJ string
    pub fn from_proj_string(proj_string: &str) -> Result<Self> {
        let proj = Proj::new(proj_string)
            .map_err(|e| Error::Projection(format!("Failed to create projection: {}", e)))?;

        Ok(Self {
            proj,
            from_epsg: 0,
            to_epsg: 0,
        })
    }

    /// Transforms a coordinate from source to target CRS
    pub fn transform(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Transformation failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Transforms multiple coordinates in bulk
    pub fn transform_many(&self, coords: &[Coordinate]) -> Result<Vec<Coordinate>> {
        coords.iter()
            .map(|&coord| self.transform(coord))
            .collect()
    }

    /// Transforms a coordinate from target back to source CRS
    pub fn transform_inverse(&self, coord: Coordinate) -> Result<Coordinate> {
        let from = format!("EPSG:{}", self.to_epsg);
        let to = format!("EPSG:{}", self.from_epsg);

        let inverse_proj = Proj::new_known_crs(&from, &to, None)
            .map_err(|e| Error::Projection(format!("Failed to create inverse projection: {}", e)))?;

        let result = inverse_proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Inverse transformation failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Returns the source EPSG code (0 if created from custom proj string)
    pub fn from_epsg(&self) -> u16 {
        self.from_epsg
    }

    /// Returns the target EPSG code (0 if created from custom proj string)
    pub fn to_epsg(&self) -> u16 {
        self.to_epsg
    }
}
