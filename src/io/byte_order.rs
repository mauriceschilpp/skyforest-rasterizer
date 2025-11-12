//! Byte order (endianness) handling
//!
//! Provides utilities for reading multi-byte values in different byte orders.
//! Supports both little-endian and big-endian formats commonly found in
//! raster file formats like TIFF.

use std::io::{self, Result};
use crate::io::SeekableReader;

/// Represents the byte order (endianness) of binary data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// Little-endian byte order (least significant byte first)
    LittleEndian,
    /// Big-endian byte order (most significant byte first)
    BigEndian,
}

impl ByteOrder {
    /// Detects byte order from TIFF magic bytes
    ///
    /// TIFF files start with either "II" (0x4949) for little-endian
    /// or "MM" (0x4D4D) for big-endian.
    pub fn from_tiff_magic(magic: [u8; 2]) -> Option<Self> {
        match &magic {
            b"II" => Some(ByteOrder::LittleEndian),
            b"MM" => Some(ByteOrder::BigEndian),
            _ => None,
        }
    }

    /// Reads and detects byte order from a reader
    ///
    /// Reads the first 2 bytes and attempts to identify the byte order
    /// based on TIFF magic number conventions.
    pub fn detect<R: SeekableReader>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 2];
        reader.read_exact(&mut magic)?;

        Self::from_tiff_magic(magic).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid byte order magic bytes: {:02X}{:02X}", magic[0], magic[1])
            )
        })
    }

    /// Creates a handler for this byte order
    pub fn handler(&self) -> Box<dyn ByteOrderHandler> {
        match self {
            ByteOrder::LittleEndian => Box::new(LittleEndian),
            ByteOrder::BigEndian => Box::new(BigEndian),
        }
    }
}

/// Trait for reading typed values with specific byte order
pub trait ByteOrderHandler: Send + Sync {
    /// Reads an unsigned 16-bit integer
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16>;

    /// Reads an unsigned 32-bit integer
    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32>;

    /// Reads an unsigned 64-bit integer
    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64>;

    /// Reads a signed 16-bit integer
    fn read_i16(&self, reader: &mut dyn SeekableReader) -> Result<i16>;

    /// Reads a signed 32-bit integer
    fn read_i32(&self, reader: &mut dyn SeekableReader) -> Result<i32>;

    /// Reads a signed 64-bit integer
    fn read_i64(&self, reader: &mut dyn SeekableReader) -> Result<i64>;

    /// Reads a 32-bit floating point number
    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32>;

    /// Reads a 64-bit floating point number
    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64>;
}

struct LittleEndian;

impl ByteOrderHandler for LittleEndian {
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn read_i16(&self, reader: &mut dyn SeekableReader) -> Result<i16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    fn read_i32(&self, reader: &mut dyn SeekableReader) -> Result<i32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i64(&self, reader: &mut dyn SeekableReader) -> Result<i64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }
}

struct BigEndian;

impl ByteOrderHandler for BigEndian {
    fn read_u16(&self, reader: &mut dyn SeekableReader) -> Result<u16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_u32(&self, reader: &mut dyn SeekableReader) -> Result<u32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64(&self, reader: &mut dyn SeekableReader) -> Result<u64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    fn read_i16(&self, reader: &mut dyn SeekableReader) -> Result<i16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    fn read_i32(&self, reader: &mut dyn SeekableReader) -> Result<i32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    fn read_i64(&self, reader: &mut dyn SeekableReader) -> Result<i64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    fn read_f32(&self, reader: &mut dyn SeekableReader) -> Result<f32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    fn read_f64(&self, reader: &mut dyn SeekableReader) -> Result<f64> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        Ok(f64::from_be_bytes(buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_from_tiff_magic_little_endian() {
        assert_eq!(
            ByteOrder::from_tiff_magic(*b"II"),
            Some(ByteOrder::LittleEndian)
        );
    }

    #[test]
    fn test_from_tiff_magic_big_endian() {
        assert_eq!(
            ByteOrder::from_tiff_magic(*b"MM"),
            Some(ByteOrder::BigEndian)
        );
    }

    #[test]
    fn test_from_tiff_magic_invalid() {
        assert_eq!(ByteOrder::from_tiff_magic(*b"XX"), None);
    }

    #[test]
    fn test_detect_little_endian() {
        let data = b"II";
        let mut cursor = Cursor::new(data);
        let order = ByteOrder::detect(&mut cursor).unwrap();
        assert_eq!(order, ByteOrder::LittleEndian);
    }

    #[test]
    fn test_detect_big_endian() {
        let data = b"MM";
        let mut cursor = Cursor::new(data);
        let order = ByteOrder::detect(&mut cursor).unwrap();
        assert_eq!(order, ByteOrder::BigEndian);
    }

    #[test]
    fn test_detect_invalid() {
        let data = b"XX";
        let mut cursor = Cursor::new(data);
        assert!(ByteOrder::detect(&mut cursor).is_err());
    }

    #[test]
    fn test_little_endian_read_u16() {
        let data = vec![0x34u8, 0x12];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let value = handler.read_u16(&mut cursor).unwrap();
        assert_eq!(value, 0x1234);
    }

    #[test]
    fn test_big_endian_read_u16() {
        let data = vec![0x12u8, 0x34];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = BigEndian;
        let value = handler.read_u16(&mut cursor).unwrap();
        assert_eq!(value, 0x1234);
    }

    #[test]
    fn test_little_endian_read_u32() {
        let data = vec![0x78u8, 0x56, 0x34, 0x12];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let value = handler.read_u32(&mut cursor).unwrap();
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_big_endian_read_u32() {
        let data = vec![0x12u8, 0x34, 0x56, 0x78];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = BigEndian;
        let value = handler.read_u32(&mut cursor).unwrap();
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_little_endian_read_u64() {
        let data = vec![0x88u8, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let value = handler.read_u64(&mut cursor).unwrap();
        assert_eq!(value, 0x1122334455667788);
    }

    #[test]
    fn test_little_endian_read_i16() {
        let data = vec![0xFFu8, 0xFF];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let value = handler.read_i16(&mut cursor).unwrap();
        assert_eq!(value, -1);
    }

    #[test]
    fn test_little_endian_read_i32() {
        let data = vec![0xFFu8, 0xFF, 0xFF, 0xFF];
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let value = handler.read_i32(&mut cursor).unwrap();
        assert_eq!(value, -1);
    }

    #[test]
    fn test_little_endian_read_f32() {
        let value = std::f32::consts::PI;
        let data = value.to_le_bytes().to_vec();
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let read_value = handler.read_f32(&mut cursor).unwrap();
        assert!((read_value - value).abs() < 0.0001);
    }

    #[test]
    fn test_big_endian_read_f32() {
        let value = std::f32::consts::PI;
        let data = value.to_be_bytes().to_vec();
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = BigEndian;
        let read_value = handler.read_f32(&mut cursor).unwrap();
        assert!((read_value - value).abs() < 0.0001);
    }

    #[test]
    fn test_little_endian_read_f64() {
        let value = std::f64::consts::PI;
        let data = value.to_le_bytes().to_vec();
        let mut cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(data));
        let handler = LittleEndian;
        let read_value = handler.read_f64(&mut cursor).unwrap();
        assert!((read_value - value).abs() < 0.0000001);
    }

    #[test]
    fn test_handler_from_byte_order() {
        let le_handler = ByteOrder::LittleEndian.handler();
        let be_handler = ByteOrder::BigEndian.handler();

        let le_data = vec![0x34u8, 0x12];
        let mut le_cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(le_data));
        assert_eq!(le_handler.read_u16(&mut le_cursor).unwrap(), 0x1234);

        let be_data = vec![0x12u8, 0x34];
        let mut be_cursor: Box<dyn SeekableReader> = Box::new(Cursor::new(be_data));
        assert_eq!(be_handler.read_u16(&mut be_cursor).unwrap(), 0x1234);
    }
}
