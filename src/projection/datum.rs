use crate::error::{Error, Result};
use crate::projection::coordinate::Coordinate;
use proj::Proj;

/// Represents common geodetic datums
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Datum {
    WGS84,
    NAD83,
    NAD27,
    ETRS89,
    GDA94,
    GDA2020,
    Custom,
}

impl Datum {
    /// Converts datum to PROJ string representation
    pub fn to_proj_string(&self) -> &str {
        match self {
            Datum::WGS84 => "+datum=WGS84",
            Datum::NAD83 => "+datum=NAD83",
            Datum::NAD27 => "+datum=NAD27",
            Datum::ETRS89 => "+datum=ETRS89",
            Datum::GDA94 => "+datum=GDA94",
            Datum::GDA2020 => "+datum=GDA2020",
            Datum::Custom => "",
        }
    }
}

/// Transforms coordinates between different geodetic datums
pub struct DatumTransform {
    proj: Proj,
    from_datum: Datum,
    to_datum: Datum,
}

impl DatumTransform {
    /// Creates a new datum transformation between two datums
    pub fn new(from_datum: Datum, to_datum: Datum) -> Result<Self> {
        let proj_string = format!(
            "+proj=longlat {} +to +proj=longlat {}",
            from_datum.to_proj_string(),
            to_datum.to_proj_string()
        );

        let proj = Proj::new(&proj_string)
            .map_err(|e| Error::Projection(format!("Failed to create datum transform: {}", e)))?;

        Ok(Self {
            proj,
            from_datum,
            to_datum,
        })
    }

    /// Creates a datum transformation from a custom PROJ string
    pub fn from_custom_string(proj_string: &str) -> Result<Self> {
        let proj = Proj::new(proj_string)
            .map_err(|e| Error::Projection(format!("Failed to create datum transform: {}", e)))?;

        Ok(Self {
            proj,
            from_datum: Datum::Custom,
            to_datum: Datum::Custom,
        })
    }

    /// Transforms a 2D coordinate between datums
    pub fn transform(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Datum transformation failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Transforms a coordinate while preserving height component
    pub fn transform_with_height(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Datum transformation failed: {}", e)))?;

        Ok(Coordinate::new_3d(result.0, result.1, coord.z))
    }

    /// Returns the source datum
    pub fn from_datum(&self) -> Datum {
        self.from_datum
    }

    /// Returns the target datum
    pub fn to_datum(&self) -> Datum {
        self.to_datum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datum_proj_string() {
        assert_eq!(Datum::WGS84.to_proj_string(), "+datum=WGS84");
        assert_eq!(Datum::NAD83.to_proj_string(), "+datum=NAD83");
        assert_eq!(Datum::NAD27.to_proj_string(), "+datum=NAD27");
    }

    #[test]
    fn test_datum_transform_creation() {
        let result = DatumTransform::new(Datum::WGS84, Datum::NAD83);
        assert!(result.is_ok());

        let transform = result.unwrap();
        assert_eq!(transform.from_datum(), Datum::WGS84);
        assert_eq!(transform.to_datum(), Datum::NAD83);
    }

    #[test]
    fn test_datum_custom_string() {
        let result = DatumTransform::from_custom_string("+proj=longlat +datum=WGS84");
        assert!(result.is_ok());
    }
}
