//! PackBits decompression
//!
//! PackBits is a simple run-length encoding scheme used in TIFF files.

use crate::error::{Error, Result};

/// Decompresses PackBits compressed data
///
/// PackBits encoding:
/// - If header >= 0: copy next (header + 1) literal bytes
/// - If header < 0 and != -128: repeat next byte (1 - header) times
/// - If header == -128: no operation (skip)
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let header = data[pos] as i8;
        pos += 1;

        match header {
            -128 => continue,

            0..=127 => {
                let count = (header as usize) + 1;

                if pos + count > data.len() {
                    return Err(Error::InvalidFormat(
                        "PackBits: Insufficient literal bytes".to_string()
                    ));
                }

                output.extend_from_slice(&data[pos..pos + count]);
                pos += count;
            }

            -127..=-1 => {
                if pos >= data.len() {
                    return Err(Error::InvalidFormat(
                        "PackBits: Missing run byte".to_string()
                    ));
                }

                let count = (1 - header as isize) as usize;
                let byte = data[pos];
                pos += 1;

                output.resize(output.len() + count, byte);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_bytes() {
        let data = vec![2, 0x41, 0x42, 0x43];
        let result = decompress(&data).unwrap();
        assert_eq!(result, vec![0x41, 0x42, 0x43]);
    }

    #[test]
    fn test_run_bytes() {
        let data = vec![(-3i8) as u8, 0xAA];
        let result = decompress(&data).unwrap();
        assert_eq!(result, vec![0xAA, 0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn test_mixed() {
        let data = vec![
            1, 0x41, 0x42,
            (-2i8) as u8, 0x55,
            0, 0x43,
        ];
        let result = decompress(&data).unwrap();
        assert_eq!(result, vec![0x41, 0x42, 0x55, 0x55, 0x55, 0x43]);
    }

    #[test]
    fn test_noop() {
        let data = vec![(-128i8) as u8, 1, 0x41, 0x42];
        let result = decompress(&data).unwrap();
        assert_eq!(result, vec![0x41, 0x42]);
    }
}
