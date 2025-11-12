//! Tag value reading operations

use std::io::{Read, Seek, SeekFrom};
use crate::error::{Error, Result};
use crate::io::BufferedReader;
use crate::io::byte_order::ByteOrderHandler;
use crate::formats::tiff::IFDEntry;

/// Handles reading tag values from TIFF files
pub struct TagReader<'a, R: Read + Seek + Send + Sync> {
    reader: &'a mut BufferedReader<R>,
    handler: &'a dyn ByteOrderHandler,
    is_big_tiff: bool,
}

impl<'a, R: Read + Seek + Send + Sync> TagReader<'a, R> {
    pub fn new(
        reader: &'a mut BufferedReader<R>,
        handler: &'a dyn ByteOrderHandler,
        is_big_tiff: bool,
    ) -> Self {
        Self {
            reader,
            handler,
            is_big_tiff,
        }
    }

    /// Reads tag values as f64 array
    pub fn read_doubles(&mut self, entry: &IFDEntry) -> Result<Vec<f64>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for _ in 0..entry.count {
            let value = self.read_single_double(entry)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as u16 array
    pub fn read_u16s(&mut self, entry: &IFDEntry) -> Result<Vec<u16>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_u16(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as i16 array
    pub fn read_i16s(&mut self, entry: &IFDEntry) -> Result<Vec<i16>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_i16(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as u32 array
    pub fn read_u32s(&mut self, entry: &IFDEntry) -> Result<Vec<u32>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_u32(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as i32 array
    pub fn read_i32s(&mut self, entry: &IFDEntry) -> Result<Vec<i32>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_i32(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as u64 array
    pub fn read_u64s(&mut self, entry: &IFDEntry) -> Result<Vec<u64>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_u64(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads tag values as i64 array
    pub fn read_i64s(&mut self, entry: &IFDEntry) -> Result<Vec<i64>> {
        self.seek_to_tag_data(entry)?;
        let mut values = Vec::with_capacity(entry.count as usize);

        for i in 0..entry.count {
            let value = self.read_single_i64(entry, i)?;
            values.push(value);
        }

        Ok(values)
    }

    /// Reads ASCII string from tag
    pub fn read_ascii(&mut self, entry: &IFDEntry) -> Result<String> {
        let bytes = self.read_ascii_bytes(entry)?;
        let s = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\0')
            .to_string();
        Ok(s)
    }

    fn read_single_double(&mut self, entry: &IFDEntry) -> Result<f64> {
        use crate::formats::tiff::tags::field_types;

        match entry.field_type {
            field_types::DOUBLE => {
                if entry.is_inline(self.is_big_tiff) {
                    return Ok(f64::from_bits(entry.value_offset));
                }
                Ok(self.handler.read_f64(self.reader)?)
            }
            field_types::FLOAT => {
                let float_val = self.handler.read_f32(self.reader)?;
                Ok(float_val as f64)
            }
            _ => Err(Error::InvalidFormat("Expected DOUBLE or FLOAT type".to_string())),
        }
    }

    fn read_single_u16(&mut self, entry: &IFDEntry, index: u64) -> Result<u16> {
        if entry.is_inline(self.is_big_tiff) {
            return Ok(((entry.value_offset >> (index * 16)) & 0xFFFF) as u16);
        }
        Ok(self.handler.read_u16(self.reader)?)
    }

    fn read_single_i16(&mut self, entry: &IFDEntry, index: u64) -> Result<i16> {
        if entry.is_inline(self.is_big_tiff) {
            return Ok(((entry.value_offset >> (index * 16)) & 0xFFFF) as i16);
        }
        Ok(self.handler.read_i16(self.reader)?)
    }

    fn read_single_u32(&mut self, entry: &IFDEntry, index: u64) -> Result<u32> {
        if !entry.is_inline(self.is_big_tiff) {
            return Ok(self.handler.read_u32(self.reader)?);
        }

        if index == 0 {
            return Ok((entry.value_offset & 0xFFFFFFFF) as u32);
        }
        Ok(((entry.value_offset >> 32) & 0xFFFFFFFF) as u32)
    }

    fn read_single_i32(&mut self, entry: &IFDEntry, index: u64) -> Result<i32> {
        if !entry.is_inline(self.is_big_tiff) {
            return Ok(self.handler.read_i32(self.reader)?);
        }

        if index == 0 {
            return Ok((entry.value_offset & 0xFFFFFFFF) as i32);
        }
        Ok(((entry.value_offset >> 32) & 0xFFFFFFFF) as i32)
    }

    fn read_single_u64(&mut self, entry: &IFDEntry, index: u64) -> Result<u64> {
        if entry.is_inline(self.is_big_tiff) && index == 0 {
            return Ok(entry.value_offset);
        }
        Ok(self.handler.read_u64(self.reader)?)
    }

    fn read_single_i64(&mut self, entry: &IFDEntry, index: u64) -> Result<i64> {
        if entry.is_inline(self.is_big_tiff) && index == 0 {
            return Ok(entry.value_offset as i64);
        }
        Ok(self.handler.read_i64(self.reader)?)
    }

    fn read_ascii_bytes(&mut self, entry: &IFDEntry) -> Result<Vec<u8>> {
        let mut bytes = vec![0u8; entry.count as usize];

        if entry.is_inline(self.is_big_tiff) {
            let inline_bytes = entry.value_offset.to_le_bytes();
            bytes.copy_from_slice(&inline_bytes[..entry.count as usize]);
            return Ok(bytes);
        }

        self.reader.seek(SeekFrom::Start(entry.value_offset))?;
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn seek_to_tag_data(&mut self, entry: &IFDEntry) -> Result<()> {
        if !entry.is_inline(self.is_big_tiff) {
            self.reader.seek(SeekFrom::Start(entry.value_offset))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::io::ByteOrder;
    use crate::formats::tiff::tags::field_types;

    fn create_test_reader(data: Vec<u8>) -> BufferedReader<Cursor<Vec<u8>>> {
        BufferedReader::new(Cursor::new(data))
    }

    #[test]
    fn test_read_u16s_inline() {
        let data = vec![0u8; 16];
        let mut reader = create_test_reader(data);
        let byte_order = ByteOrder::LittleEndian;
        let handler = byte_order.handler();

        let entry = IFDEntry::new(256, field_types::SHORT, 2, 0x00020001);
        let mut tag_reader = TagReader::new(&mut reader, &*handler, false);

        let values = tag_reader.read_u16s(&entry).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], 0x0001);
        assert_eq!(values[1], 0x0002);
    }

    #[test]
    fn test_read_u32s_inline() {
        let data = vec![0u8; 16];
        let mut reader = create_test_reader(data);
        let byte_order = ByteOrder::LittleEndian;
        let handler = byte_order.handler();

        let entry = IFDEntry::new(256, field_types::LONG, 1, 0x12345678);
        let mut tag_reader = TagReader::new(&mut reader, &*handler, false);

        let values = tag_reader.read_u32s(&entry).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], 0x12345678);
    }

    #[test]
    fn test_read_i16s_inline() {
        let data = vec![0u8; 16];
        let mut reader = create_test_reader(data);
        let byte_order = ByteOrder::LittleEndian;
        let handler = byte_order.handler();

        let entry = IFDEntry::new(256, field_types::SSHORT, 2, 0xFFFE0001);
        let mut tag_reader = TagReader::new(&mut reader, &*handler, false);

        let values = tag_reader.read_i16s(&entry).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], 1);
        assert_eq!(values[1], -2_i16);
    }

    #[test]
    fn test_read_ascii_inline() {
        let data = vec![0u8; 16];
        let mut reader = create_test_reader(data);
        let byte_order = ByteOrder::LittleEndian;
        let handler = byte_order.handler();

        let text = "Hi";
        let mut value_offset = 0u64;
        value_offset |= text.as_bytes()[0] as u64;
        value_offset |= (text.as_bytes()[1] as u64) << 8;

        let entry = IFDEntry::new(256, field_types::ASCII, 2, value_offset);
        let mut tag_reader = TagReader::new(&mut reader, &*handler, false);

        let result = tag_reader.read_ascii(&entry).unwrap();
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_read_doubles_non_inline() {
        let mut data = vec![];
        data.extend_from_slice(&0f64.to_le_bytes());
        data.extend_from_slice(&1f64.to_le_bytes());
        data.extend_from_slice(&2.5f64.to_le_bytes());

        let mut reader = create_test_reader(data);
        let byte_order = ByteOrder::LittleEndian;
        let handler = byte_order.handler();

        let entry = IFDEntry::new(256, field_types::DOUBLE, 3, 0);
        let mut tag_reader = TagReader::new(&mut reader, &*handler, false);

        let values = tag_reader.read_doubles(&entry).unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], 0.0);
        assert_eq!(values[1], 1.0);
        assert_eq!(values[2], 2.5);
    }
}
