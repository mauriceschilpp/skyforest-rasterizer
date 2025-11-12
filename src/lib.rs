//! rasterkit - A modern GeoTIFF library for Rust
//!
//! rasterkit provides efficient reading and processing of GeoTIFF files
//! with support for coordinate reference systems, projections, and raster operations.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use rasterkit::TiffReader;
//!
//! let mut reader = TiffReader::open("image.tif")?;
//! let tiff = reader.read()?;
//!
//! if let Some(ifd) = tiff.main_ifd() {
//!     let dims = ifd.dimensions().unwrap();
//!     println!("Size: {} x {}", dims.width, dims.height);
//! }
//! # Ok::<(), rasterkit::Error>(())
//! ```
//!
//! ## Reading Different Data Types
//!
//! ```no_run
//! use rasterkit::{TiffReader, DataType};
//!
//! let mut reader = TiffReader::open("image.tif")?;
//! let tiff = reader.read()?;
//!
//! if let Some(ifd) = tiff.main_ifd() {
//!     match ifd.data_type() {
//!         Some(DataType::U8) => {
//!             let value = reader.read_pixel_value(ifd, 100, 100)?;
//!             println!("U8 pixel: {}", value);
//!         },
//!         Some(DataType::I16) => {
//!             let value = reader.read_pixel_i16(ifd, 100, 100)?;
//!             println!("I16 pixel (elevation): {}", value);
//!         },
//!         Some(DataType::F32) => {
//!             let value = reader.read_pixel_f32(ifd, 100, 100)?;
//!             println!("F32 pixel (scientific data): {}", value);
//!         },
//!         _ => println!("Unsupported data type"),
//!     }
//! }
//! # Ok::<(), rasterkit::Error>(())
//! ```

pub mod io;
pub mod error;
pub mod types;
pub mod formats;
pub mod compression;
pub mod cache;
pub mod cache_prefetch;
pub mod cache_prefetch_async;
pub mod projection;
pub mod api;

pub use error::{Error, Result};
pub use types::{DataType, Dimensions};
pub use formats::tiff::{
    Tiff, TiffReader, IFD, IFDEntry, GeoInfo,
    tags, TIFF_MAGIC, BIGTIFF_MAGIC
};
pub use io::{ByteOrder, BufferedReader, SeekableReader};
pub use projection::{Coordinate, Transformer, Datum, DatumTransform, GridShift, CustomProjection};
