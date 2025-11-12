//! Compression and decompression utilities

pub mod deflate;
pub mod lzw;
pub mod packbits;
pub mod jpeg;

use crate::error::{Error, Result};

/// Compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// No compression
    None,
    /// Deflate/ZIP compression
    Deflate,
    /// LZW compression
    Lzw,
    /// PackBits compression
    PackBits,
    /// JPEG compression
    Jpeg,
}

impl Compression {
    /// Creates compression from TIFF compression tag value
    pub fn from_tag(value: u64) -> Result<Self> {
        match value {
            1 => Ok(Compression::None),
            5 => Ok(Compression::Lzw),
            8 => Ok(Compression::Deflate),
            32773 => Ok(Compression::PackBits),
            7 => Ok(Compression::Jpeg),
            _ => Err(Error::Unsupported(format!("Compression type {}", value))),
        }
    }

    /// Returns the name of this compression type
    pub fn name(&self) -> &'static str {
        match self {
            Compression::None => "None",
            Compression::Deflate => "Deflate/ZIP",
            Compression::Lzw => "LZW",
            Compression::PackBits => "PackBits",
            Compression::Jpeg => "JPEG",
        }
    }

    /// Decompresses data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Compression::None => Ok(data.to_vec()),
            Compression::Deflate => deflate::decompress(data),
            Compression::Lzw => lzw::decompress(data),
            Compression::PackBits => packbits::decompress(data),
            Compression::Jpeg => jpeg::decompress(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_from_tag() {
        assert_eq!(Compression::from_tag(1).unwrap(), Compression::None);
        assert_eq!(Compression::from_tag(8).unwrap(), Compression::Deflate);
        assert_eq!(Compression::from_tag(5).unwrap(), Compression::Lzw);
    }

    #[test]
    fn test_compression_name() {
        assert_eq!(Compression::None.name(), "None");
        assert_eq!(Compression::Deflate.name(), "Deflate/ZIP");
    }

    #[test]
    fn test_no_compression() {
        let data = vec![1u8, 2, 3, 4];
        let result = Compression::None.decompress(&data).unwrap();
        assert_eq!(result, data);
    }
}
