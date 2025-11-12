//! I/O utilities for rasterkit
//!
//! Provides core I/O primitives for reading raster data formats.

pub mod traits;
pub mod byte_order;
pub mod buffer;

pub use traits::SeekableReader;
pub use byte_order::ByteOrder;
pub use buffer::BufferedReader;
