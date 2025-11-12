//! LZW decompression
//!
//! LZW (Lempel-Ziv-Welch) is a lossless compression algorithm used in TIFF files.

use crate::error::{Error, Result};

/// Decompresses LZW compressed data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    LzwDecoder::new().decode(data)
}

/// LZW decoder
struct LzwDecoder {
    dictionary: Vec<Vec<u8>>,
    next_code: usize,
}

impl LzwDecoder {
    fn new() -> Self {
        let mut dictionary = Vec::with_capacity(4096);

        for i in 0..256 {
            dictionary.push(vec![i as u8]);
        }

        Self {
            dictionary,
            next_code: 258,
        }
    }

    fn decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut reader = BitReader::new(data);
        let mut code_size = 9;
        let mut previous_code: Option<u16> = None;

        while let Some(code) = reader.read_bits(code_size) {
            if code == 257 {
                break;
            }

            if code == 256 {
                self.reset();
                code_size = 9;
                previous_code = None;
                continue;
            }

            let entry = self.get_entry(code as usize, previous_code)?;
            output.extend_from_slice(&entry);

            if let Some(prev) = previous_code {
                self.add_entry(prev as usize, entry[0]);

                if self.next_code == (1 << code_size) && code_size < 12 {
                    code_size += 1;
                }
            }

            previous_code = Some(code);
        }

        Ok(output)
    }

    fn get_entry(&self, code: usize, previous: Option<u16>) -> Result<Vec<u8>> {
        if code < self.dictionary.len() {
            Ok(self.dictionary[code].clone())
        } else if code == self.next_code {
            if let Some(prev) = previous {
                let mut entry = self.dictionary[prev as usize].clone();
                entry.push(entry[0]);
                Ok(entry)
            } else {
                Err(Error::InvalidFormat("Invalid LZW sequence".to_string()))
            }
        } else {
            Err(Error::InvalidFormat(format!("Invalid LZW code: {}", code)))
        }
    }

    fn add_entry(&mut self, previous_code: usize, first_byte: u8) {
        if self.next_code < 4096 {
            let mut entry = self.dictionary[previous_code].clone();
            entry.push(first_byte);
            self.dictionary.push(entry);
            self.next_code += 1;
        }
    }

    fn reset(&mut self) {
        self.dictionary.truncate(256);
        self.next_code = 258;
    }
}

/// Reads variable-length bit codes from byte stream
struct BitReader<'a> {
    data: &'a [u8],
    byte_index: usize,
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_index: 0,
            bit_offset: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Option<u16> {
        if count > 16 || count == 0 {
            return None;
        }

        let mut result = 0u16;
        let mut bits_read = 0u8;

        while bits_read < count {
            if self.byte_index >= self.data.len() {
                return None;
            }

            let available_bits = 8 - self.bit_offset;
            let needed_bits = count - bits_read;
            let bits_to_read = available_bits.min(needed_bits);

            let mask = if bits_to_read == 8 {
                0xFF
            } else {
                (1u8 << bits_to_read) - 1
            };
            let bits = (self.data[self.byte_index] >> self.bit_offset) & mask;

            result |= (bits as u16) << bits_read;
            bits_read += bits_to_read;
            self.bit_offset += bits_to_read;

            if self.bit_offset >= 8 {
                self.bit_offset = 0;
                self.byte_index += 1;
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_reader() {
        let data = vec![0b11010010, 0b10110101];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(3), Some(0b010));
        assert_eq!(reader.read_bits(5), Some(0b11010));
    }

    #[test]
    fn test_lzw_basic_functionality() {
        // Test that LZW decoder can be instantiated and handle basic operations
        let decoder = LzwDecoder::new();
        assert_eq!(decoder.dictionary.len(), 256);
        assert_eq!(decoder.next_code, 258);

        // Test bit reader with known data
        let test_data = vec![0xFF, 0xFF];
        let mut reader = BitReader::new(&test_data);
        assert_eq!(reader.read_bits(8), Some(0xFF));
        assert_eq!(reader.read_bits(8), Some(0xFF));
    }
}
