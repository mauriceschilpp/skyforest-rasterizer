/// Represents a coordinate in any coordinate reference system
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coordinate {
    /// Creates a new 2D coordinate (z = 0.0)
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, z: 0.0 }
    }

    /// Creates a new 3D coordinate
    pub fn new_3d(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Creates a coordinate from longitude/latitude in degrees (WGS84)
    pub fn from_lonlat(lon: f64, lat: f64) -> Self {
        Self::new(lon, lat)
    }

    /// Creates a 3D coordinate from longitude/latitude/altitude
    pub fn from_lonlat_alt(lon: f64, lat: f64, alt: f64) -> Self {
        Self::new_3d(lon, lat, alt)
    }
}
