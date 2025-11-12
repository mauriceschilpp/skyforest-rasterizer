//! TIFF tag constants

/// Image width in pixels
pub const IMAGE_WIDTH: u16 = 256;

/// Image height in pixels
pub const IMAGE_LENGTH: u16 = 257;

/// Bits per sample
pub const BITS_PER_SAMPLE: u16 = 258;

/// Compression scheme
pub const COMPRESSION: u16 = 259;

/// Photometric interpretation
pub const PHOTOMETRIC_INTERPRETATION: u16 = 262;

/// Image description
pub const IMAGE_DESCRIPTION: u16 = 270;

/// Strip offsets
pub const STRIP_OFFSETS: u16 = 273;

/// Samples per pixel
pub const SAMPLES_PER_PIXEL: u16 = 277;

/// Rows per strip
pub const ROWS_PER_STRIP: u16 = 278;

/// Strip byte counts
pub const STRIP_BYTE_COUNTS: u16 = 279;

/// X resolution
pub const X_RESOLUTION: u16 = 282;

/// Y resolution
pub const Y_RESOLUTION: u16 = 283;

/// Planar configuration
pub const PLANAR_CONFIGURATION: u16 = 284;

/// Resolution unit
pub const RESOLUTION_UNIT: u16 = 296;

/// Software
pub const SOFTWARE: u16 = 305;

/// Predictor
pub const PREDICTOR: u16 = 317;

/// Date/time
pub const DATE_TIME: u16 = 306;

/// Tile width
pub const TILE_WIDTH: u16 = 322;

/// Tile length
pub const TILE_LENGTH: u16 = 323;

/// Tile offsets
pub const TILE_OFFSETS: u16 = 324;

/// Tile byte counts
pub const TILE_BYTE_COUNTS: u16 = 325;

/// Sample format
pub const SAMPLE_FORMAT: u16 = 339;

/// GeoTIFF ModelPixelScaleTag
pub const MODEL_PIXEL_SCALE: u16 = 33550;

/// GeoTIFF ModelTiepointTag
pub const MODEL_TIEPOINT: u16 = 33922;

/// GeoTIFF ModelTransformationTag
pub const MODEL_TRANSFORMATION: u16 = 34264;

/// GeoTIFF GeoKeyDirectoryTag
pub const GEO_KEY_DIRECTORY: u16 = 34735;

/// GeoTIFF GeoDoubleParamsTag
pub const GEO_DOUBLE_PARAMS: u16 = 34736;

/// GeoTIFF GeoAsciiParamsTag
pub const GEO_ASCII_PARAMS: u16 = 34737;

/// GDAL metadata
pub const GDAL_METADATA: u16 = 42112;

/// GDAL no data value
pub const GDAL_NODATA: u16 = 42113;

/// Returns the name of a TIFF tag
pub fn tag_name(tag: u16) -> &'static str {
    match tag {
        IMAGE_WIDTH => "ImageWidth",
        IMAGE_LENGTH => "ImageLength",
        BITS_PER_SAMPLE => "BitsPerSample",
        COMPRESSION => "Compression",
        PHOTOMETRIC_INTERPRETATION => "PhotometricInterpretation",
        IMAGE_DESCRIPTION => "ImageDescription",
        STRIP_OFFSETS => "StripOffsets",
        SAMPLES_PER_PIXEL => "SamplesPerPixel",
        ROWS_PER_STRIP => "RowsPerStrip",
        STRIP_BYTE_COUNTS => "StripByteCounts",
        X_RESOLUTION => "XResolution",
        Y_RESOLUTION => "YResolution",
        PLANAR_CONFIGURATION => "PlanarConfiguration",
        RESOLUTION_UNIT => "ResolutionUnit",
        SOFTWARE => "Software",
        DATE_TIME => "DateTime",
        TILE_WIDTH => "TileWidth",
        TILE_LENGTH => "TileLength",
        TILE_OFFSETS => "TileOffsets",
        TILE_BYTE_COUNTS => "TileByteCounts",
        SAMPLE_FORMAT => "SampleFormat",
        MODEL_PIXEL_SCALE => "ModelPixelScale",
        MODEL_TIEPOINT => "ModelTiepoint",
        MODEL_TRANSFORMATION => "ModelTransformation",
        GEO_KEY_DIRECTORY => "GeoKeyDirectory",
        GEO_DOUBLE_PARAMS => "GeoDoubleParams",
        GEO_ASCII_PARAMS => "GeoAsciiParams",
        GDAL_METADATA => "GDAL_METADATA",
        GDAL_NODATA => "GDAL_NODATA",
        _ => "Unknown",
    }
}

/// Field type constants
pub mod field_types {
    /// BYTE (8-bit unsigned)
    pub const BYTE: u16 = 1;

    /// ASCII string
    pub const ASCII: u16 = 2;

    /// SHORT (16-bit unsigned)
    pub const SHORT: u16 = 3;

    /// LONG (32-bit unsigned)
    pub const LONG: u16 = 4;

    /// RATIONAL (two LONGs: numerator, denominator)
    pub const RATIONAL: u16 = 5;

    /// SBYTE (8-bit signed)
    pub const SBYTE: u16 = 6;

    /// UNDEFINED (8-bit)
    pub const UNDEFINED: u16 = 7;

    /// SSHORT (16-bit signed)
    pub const SSHORT: u16 = 8;

    /// SLONG (32-bit signed)
    pub const SLONG: u16 = 9;

    /// SRATIONAL (two SLONGs)
    pub const SRATIONAL: u16 = 10;

    /// FLOAT (32-bit IEEE float)
    pub const FLOAT: u16 = 11;

    /// DOUBLE (64-bit IEEE double)
    pub const DOUBLE: u16 = 12;

    /// LONG8 (64-bit unsigned, BigTIFF)
    pub const LONG8: u16 = 16;

    /// SLONG8 (64-bit signed, BigTIFF)
    pub const SLONG8: u16 = 17;

    /// IFD8 (64-bit IFD offset, BigTIFF)
    pub const IFD8: u16 = 18;
}

/// Returns the name of a field type
pub fn field_type_name(field_type: u16) -> &'static str {
    match field_type {
        field_types::BYTE => "BYTE",
        field_types::ASCII => "ASCII",
        field_types::SHORT => "SHORT",
        field_types::LONG => "LONG",
        field_types::RATIONAL => "RATIONAL",
        field_types::SBYTE => "SBYTE",
        field_types::UNDEFINED => "UNDEFINED",
        field_types::SSHORT => "SSHORT",
        field_types::SLONG => "SLONG",
        field_types::SRATIONAL => "SRATIONAL",
        field_types::FLOAT => "FLOAT",
        field_types::DOUBLE => "DOUBLE",
        field_types::LONG8 => "LONG8",
        field_types::SLONG8 => "SLONG8",
        field_types::IFD8 => "IFD8",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_name() {
        assert_eq!(tag_name(IMAGE_WIDTH), "ImageWidth");
        assert_eq!(tag_name(COMPRESSION), "Compression");
        assert_eq!(tag_name(9999), "Unknown");
    }

    #[test]
    fn test_field_type_name() {
        assert_eq!(field_type_name(field_types::BYTE), "BYTE");
        assert_eq!(field_type_name(field_types::SHORT), "SHORT");
        assert_eq!(field_type_name(field_types::LONG8), "LONG8");
        assert_eq!(field_type_name(9999), "Unknown");
    }
}
