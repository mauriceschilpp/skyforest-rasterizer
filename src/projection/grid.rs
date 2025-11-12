use crate::error::{Error, Result};
use crate::projection::coordinate::Coordinate;
use std::path::Path;

/// Applies grid-based coordinate shifts for high-accuracy transformations
pub struct GridShift {
    grid_path: String,
}

impl GridShift {
    /// Creates a new grid shift from a grid file path
    pub fn new<P: AsRef<Path>>(grid_path: P) -> Result<Self> {
        let path_str = grid_path.as_ref()
            .to_str()
            .ok_or_else(|| Error::Projection("Invalid grid path".to_string()))?
            .to_string();

        if !Path::new(&path_str).exists() {
            return Err(Error::Projection(format!("Grid file not found: {}", path_str)));
        }

        Ok(Self {
            grid_path: path_str,
        })
    }

    /// Applies the grid shift to a coordinate
    pub fn apply(&self, coord: Coordinate) -> Result<Coordinate> {
        let proj_string = format!("+proj=pipeline +step +proj=hgridshift +grids={}", self.grid_path);
        let proj = proj::Proj::new(&proj_string)
            .map_err(|e| Error::Projection(format!("Failed to apply grid shift: {}", e)))?;

        let result = proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Grid shift transformation failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Returns the path to the grid file
    pub fn grid_path(&self) -> &str {
        &self.grid_path
    }
}

/// Builder for complex grid shift pipelines with multiple grids
pub struct GridShiftBuilder {
    grid_files: Vec<String>,
    use_vgrid: bool,
}

impl GridShiftBuilder {
    /// Creates a new grid shift builder
    pub fn new() -> Self {
        Self {
            grid_files: Vec::new(),
            use_vgrid: false,
        }
    }

    /// Adds a horizontal grid shift to the pipeline
    pub fn add_horizontal_grid<P: AsRef<Path>>(mut self, grid_path: P) -> Result<Self> {
        let path_str = grid_path.as_ref()
            .to_str()
            .ok_or_else(|| Error::Projection("Invalid grid path".to_string()))?
            .to_string();

        self.grid_files.push(format!("+step +proj=hgridshift +grids={}", path_str));
        Ok(self)
    }

    /// Adds a vertical grid shift to the pipeline
    pub fn add_vertical_grid<P: AsRef<Path>>(mut self, grid_path: P) -> Result<Self> {
        let path_str = grid_path.as_ref()
            .to_str()
            .ok_or_else(|| Error::Projection("Invalid grid path".to_string()))?
            .to_string();

        self.grid_files.push(format!("+step +proj=vgridshift +grids={}", path_str));
        self.use_vgrid = true;
        Ok(self)
    }

    /// Builds the complex grid shift transformation
    pub fn build(self) -> Result<ComplexGridShift> {
        if self.grid_files.is_empty() {
            return Err(Error::Projection("No grid files specified".to_string()));
        }

        let proj_string = format!("+proj=pipeline {}", self.grid_files.join(" "));
        let proj = proj::Proj::new(&proj_string)
            .map_err(|e| Error::Projection(format!("Failed to create grid shift pipeline: {}", e)))?;

        Ok(ComplexGridShift {
            proj,
            has_vgrid: self.use_vgrid,
        })
    }
}

impl Default for GridShiftBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Complex grid shift with multiple horizontal and/or vertical grids
pub struct ComplexGridShift {
    proj: proj::Proj,
    has_vgrid: bool,
}

impl ComplexGridShift {
    /// Transforms a 2D coordinate using the grid shift pipeline
    pub fn transform(&self, coord: Coordinate) -> Result<Coordinate> {
        let result = self.proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Grid shift transformation failed: {}", e)))?;

        Ok(Coordinate::new(result.0, result.1))
    }

    /// Transforms a 3D coordinate using vertical grid shifts
    pub fn transform_3d(&self, coord: Coordinate) -> Result<Coordinate> {
        if !self.has_vgrid {
            return Err(Error::Projection("No vertical grid available for 3D transformation".to_string()));
        }

        let result = self.proj.convert((coord.x, coord.y))
            .map_err(|e| Error::Projection(format!("Grid shift 3D transformation failed: {}", e)))?;

        Ok(Coordinate::new_3d(result.0, result.1, coord.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_shift_builder() {
        let builder = GridShiftBuilder::new();
        assert_eq!(builder.grid_files.len(), 0);
        assert!(!builder.use_vgrid);
    }

    #[test]
    fn test_grid_shift_builder_empty_build() {
        let builder = GridShiftBuilder::new();
        let result = builder.build();
        assert!(result.is_err());
    }
}
