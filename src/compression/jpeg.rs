//! JPEG decompression for TIFF files

use crate::error::{Error, Result};

/// Decompresses JPEG compressed data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = jpeg_decoder::Decoder::new(data);

    decoder.decode()
        .map_err(|e| Error::InvalidFormat(format!("JPEG error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_jpeg() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert!(decompress(&data).is_err());
    }
}
