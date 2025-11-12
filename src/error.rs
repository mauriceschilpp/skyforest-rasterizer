//! Error types for rasterkit

use std::fmt;
use std::io;

/// Result type for rasterkit operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in rasterkit operations
#[derive(Debug)]
pub enum Error {
    /// I/O error
    Io(io::Error),

    /// Invalid TIFF format
    InvalidFormat(String),

    /// Invalid byte order
    InvalidByteOrder(u16),

    /// Invalid TIFF magic number
    InvalidMagic(u16),

    /// Invalid tag
    InvalidTag(u16),

    /// Missing required tag
    MissingTag(u16),

    /// Unsupported feature
    Unsupported(String),

    /// Invalid IFD offset
    InvalidOffset(u64),

    /// Out of bounds access
    OutOfBounds(String),

    /// Projection error
    Projection(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            Error::InvalidByteOrder(value) => write!(f, "Invalid byte order: 0x{:04X}", value),
            Error::InvalidMagic(value) => write!(f, "Invalid TIFF magic number: {}", value),
            Error::InvalidTag(tag) => write!(f, "Invalid tag: {}", tag),
            Error::MissingTag(tag) => write!(f, "Missing required tag: {}", tag),
            Error::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            Error::InvalidOffset(offset) => write!(f, "Invalid offset: {}", offset),
            Error::OutOfBounds(msg) => write!(f, "Out of bounds: {}", msg),
            Error::Projection(msg) => write!(f, "Projection error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::InvalidFormat("test".to_string());
        assert_eq!(err.to_string(), "Invalid format: test");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_invalid_byte_order() {
        let err = Error::InvalidByteOrder(0x1234);
        assert!(err.to_string().contains("0x1234"));
    }

    #[test]
    fn test_missing_tag() {
        let err = Error::MissingTag(256);
        assert!(err.to_string().contains("256"));
    }
}
