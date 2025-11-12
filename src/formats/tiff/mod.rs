//! TIFF and BigTIFF format support

pub mod tags;
pub mod ifd;
pub mod types;
pub mod reader;
pub mod geotiff;

pub use ifd::{IFD, IFDEntry};
pub use types::Tiff;
pub use reader::TiffReader;
pub use geotiff::GeoInfo;

/// TIFF magic number (42)
pub const TIFF_MAGIC: u16 = 42;

/// BigTIFF magic number (43)
pub const BIGTIFF_MAGIC: u16 = 43;
