use crate::error::{Error, Result};
use crate::projection::coordinate::Coordinate;
use proj::Proj;

/// Represents a custom projection definition
pub struct CustomProjection {
    proj: Proj,
    definition: String,
}

impl CustomProjection {
    /// Creates a custom projection from a PROJ4 string
    pub fn from_proj4(proj4_string: &str) -> Result<Self> {
        let proj = Proj::new(proj4_string)
            .map_err(|e| Error::Projection(format!("Failed to create custom projection: {}", e)))?;

        Ok(Self {
            proj,
            definition: proj4_string.to_string(),
        })
    }

    /// Creates a custom projection from a WKT string
    pub fn from_wkt(wkt_string: &str) -> Result<Self> {
        let proj = Proj::new(wkt_string)
            .map_err(|e| Error::Projection(format!("Failed to create custom projection from WKT: {}", e)))?;

        Ok(Self {
            proj,
            definition: wkt_string.to_string(),
        })
    }

    /// Projects a coordinate from geographic to projected coordinates
    pub fn project(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.project((coord.x, coord.y), false)
            .map_err(|e| Error::Projection(format!("Projection failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Unprojects a coordinate from projected to geographic coordinates
    pub fn unproject(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.project((coord.x, coord.y), true)
            .map_err(|e| Error::Projection(format!("Unprojection failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Returns the projection definition string
    pub fn definition(&self) -> &str {
        &self.definition
    }
}

/// Builder for creating custom projection definitions
pub struct ProjectionBuilder {
    proj_type: Option<String>,
    ellipsoid: Option<String>,
    datum: Option<String>,
    parameters: Vec<(String, String)>,
}

impl ProjectionBuilder {
    /// Creates a new projection builder
    pub fn new() -> Self {
        Self {
            proj_type: None,
            ellipsoid: None,
            datum: None,
            parameters: Vec::new(),
        }
    }

    /// Sets the projection type (e.g., "utm", "tmerc", "lcc")
    pub fn projection_type(mut self, proj_type: &str) -> Self {
        self.proj_type = Some(proj_type.to_string());
        self
    }

    /// Sets the ellipsoid (e.g., "WGS84", "GRS80")
    pub fn ellipsoid(mut self, ellipsoid: &str) -> Self {
        self.ellipsoid = Some(ellipsoid.to_string());
        self
    }

    /// Sets the datum (e.g., "WGS84", "NAD83")
    pub fn datum(mut self, datum: &str) -> Self {
        self.datum = Some(datum.to_string());
        self
    }

    /// Adds a custom parameter
    pub fn parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.push((key.to_string(), value.to_string()));
        self
    }

    /// Sets the latitude of origin
    pub fn latitude_of_origin(self, lat: f64) -> Self {
        self.parameter("lat_0", &lat.to_string())
    }

    /// Sets the central meridian
    pub fn central_meridian(self, lon: f64) -> Self {
        self.parameter("lon_0", &lon.to_string())
    }

    /// Sets the scale factor
    pub fn scale_factor(self, k: f64) -> Self {
        self.parameter("k", &k.to_string())
    }

    /// Sets the false easting
    pub fn false_easting(self, x: f64) -> Self {
        self.parameter("x_0", &x.to_string())
    }

    /// Sets the false northing
    pub fn false_northing(self, y: f64) -> Self {
        self.parameter("y_0", &y.to_string())
    }

    /// Sets the units (e.g., "m", "ft", "us-ft")
    pub fn units(self, units: &str) -> Self {
        self.parameter("units", units)
    }

    /// Builds the custom projection
    pub fn build(self) -> Result<CustomProjection> {
        if self.proj_type.is_none() {
            return Err(Error::Projection("Projection type is required".to_string()));
        }

        let mut parts = vec![format!("+proj={}", self.proj_type.unwrap())];

        if let Some(ellipsoid) = self.ellipsoid {
            parts.push(format!("+ellps={}", ellipsoid));
        }

        if let Some(datum) = self.datum {
            parts.push(format!("+datum={}", datum));
        }

        for (key, value) in self.parameters {
            parts.push(format!("+{}={}", key, value));
        }

        let proj4_string = parts.join(" ");
        CustomProjection::from_proj4(&proj4_string)
    }
}

impl Default for ProjectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_builder_basic() {
        let builder = ProjectionBuilder::new()
            .projection_type("utm")
            .parameter("zone", "33")
            .ellipsoid("WGS84");

        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_projection_builder_no_type() {
        let builder = ProjectionBuilder::new();
        let result = builder.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_projection_builder_utm() {
        let projection = ProjectionBuilder::new()
            .projection_type("utm")
            .parameter("zone", "33")
            .parameter("north", "")
            .ellipsoid("WGS84")
            .build()
            .unwrap();

        assert!(projection.definition().contains("+proj=utm"));
        assert!(projection.definition().contains("+zone=33"));
    }

    #[test]
    fn test_projection_builder_transverse_mercator() {
        let projection = ProjectionBuilder::new()
            .projection_type("tmerc")
            .latitude_of_origin(0.0)
            .central_meridian(15.0)
            .scale_factor(0.9996)
            .false_easting(500000.0)
            .false_northing(0.0)
            .ellipsoid("WGS84")
            .build()
            .unwrap();

        assert!(projection.definition().contains("+proj=tmerc"));
        assert!(projection.definition().contains("+lon_0=15"));
    }
}
