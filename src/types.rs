//! Core data types for rasterkit

/// Represents pixel data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// Unsigned 8-bit integer
    U8,
    /// Unsigned 16-bit integer
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Signed 8-bit integer
    I8,
    /// Signed 16-bit integer
    I16,
    /// Signed 32-bit integer
    I32,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
}

impl DataType {
    /// Returns the size in bytes for this data type
    pub fn size(&self) -> usize {
        match self {
            DataType::U8 | DataType::I8 => 1,
            DataType::U16 | DataType::I16 => 2,
            DataType::U32 | DataType::I32 | DataType::F32 => 4,
            DataType::F64 => 8,
        }
    }

    /// Returns the name of this data type
    pub fn name(&self) -> &'static str {
        match self {
            DataType::U8 => "U8",
            DataType::U16 => "U16",
            DataType::U32 => "U32",
            DataType::I8 => "I8",
            DataType::I16 => "I16",
            DataType::I32 => "I32",
            DataType::F32 => "F32",
            DataType::F64 => "F64",
        }
    }
}

/// Represents image dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    /// Width in pixels
    pub width: u64,
    /// Height in pixels
    pub height: u64,
}

impl Dimensions {
    /// Creates new dimensions
    pub fn new(width: u64, height: u64) -> Self {
        Self { width, height }
    }

    /// Returns the total number of pixels
    pub fn pixel_count(&self) -> u64 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_size() {
        assert_eq!(DataType::U8.size(), 1);
        assert_eq!(DataType::U16.size(), 2);
        assert_eq!(DataType::U32.size(), 4);
        assert_eq!(DataType::F32.size(), 4);
        assert_eq!(DataType::F64.size(), 8);
    }

    #[test]
    fn test_data_type_name() {
        assert_eq!(DataType::U8.name(), "U8");
        assert_eq!(DataType::F32.name(), "F32");
    }

    #[test]
    fn test_dimensions() {
        let dims = Dimensions::new(100, 200);
        assert_eq!(dims.width, 100);
        assert_eq!(dims.height, 200);
        assert_eq!(dims.pixel_count(), 20000);
    }
}
