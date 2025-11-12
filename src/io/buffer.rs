//! Buffered reading utilities
//!
//! Provides efficient buffered I/O operations for reading large raster files.

use std::io::{Read, Result, Seek, SeekFrom};
use crate::io::SeekableReader;

/// A buffered reader that wraps any [`SeekableReader`]
///
/// Provides efficient reading by maintaining an internal buffer,
/// reducing the number of system calls for small reads.
pub struct BufferedReader<R: SeekableReader> {
    inner: R,
    buffer: Vec<u8>,
    pos: usize,
    cap: usize,
    buffer_size: usize,
}

impl<R: SeekableReader> BufferedReader<R> {
    /// Creates a new buffered reader with default buffer size (8KB)
    pub fn new(inner: R) -> Self {
        Self::with_capacity(8192, inner)
    }

    /// Creates a new buffered reader with specified buffer size
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            inner,
            buffer: vec![0; capacity],
            pos: 0,
            cap: 0,
            buffer_size: capacity,
        }
    }

    /// Returns a reference to the underlying reader
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Consumes the buffered reader and returns the underlying reader
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Returns the number of bytes currently buffered
    pub fn buffer_len(&self) -> usize {
        self.cap - self.pos
    }

    /// Fills the internal buffer by reading from the underlying reader
    fn fill_buffer(&mut self) -> Result<()> {
        self.cap = self.inner.read(&mut self.buffer)?;
        self.pos = 0;
        Ok(())
    }

    /// Reads a chunk of specified size from the reader
    ///
    /// Returns a vector containing exactly `size` bytes, or an error
    /// if not enough data is available.
    pub fn read_chunk(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut chunk = vec![0u8; size];
        self.read_exact(&mut chunk)?;
        Ok(chunk)
    }
}

impl<R: SeekableReader> Read for BufferedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.pos >= self.cap {
            if buf.len() >= self.buffer_size {
                return self.inner.read(buf);
            }
            self.fill_buffer()?;
            if self.cap == 0 {
                return Ok(0);
            }
        }

        let available = self.cap - self.pos;
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&self.buffer[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

impl<R: SeekableReader> Seek for BufferedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.pos = 0;
        self.cap = 0;
        self.inner.seek(pos)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_buffered_reader_new() {
        let data = vec![1u8, 2, 3, 4];
        let cursor = Cursor::new(data);
        let reader = BufferedReader::new(cursor);
        assert_eq!(reader.buffer_len(), 0);
    }

    #[test]
    fn test_buffered_reader_with_capacity() {
        let data = vec![1u8, 2, 3, 4];
        let cursor = Cursor::new(data);
        let reader = BufferedReader::with_capacity(16, cursor);
        assert_eq!(reader.buffer_size, 16);
    }

    #[test]
    fn test_read_single_byte() {
        let data = vec![0x42u8];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        let mut buf = [0u8; 1];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], 0x42);
    }

    #[test]
    fn test_read_multiple_bytes() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x10, 0x20, 0x30, 0x40]);
    }

    #[test]
    fn test_read_in_chunks() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x10, 0x20]);

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x30, 0x40]);
    }

    #[test]
    fn test_seek_and_read() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        reader.seek(SeekFrom::Start(2)).unwrap();

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x30, 0x40]);
    }

    #[test]
    fn test_seek_from_end() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        reader.seek(SeekFrom::End(-2)).unwrap();

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x30, 0x40]);
    }

    #[test]
    fn test_read_chunk() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40, 0x50];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        let chunk = reader.read_chunk(3).unwrap();
        assert_eq!(chunk, vec![0x10, 0x20, 0x30]);

        let chunk = reader.read_chunk(2).unwrap();
        assert_eq!(chunk, vec![0x40, 0x50]);
    }

    #[test]
    fn test_read_chunk_insufficient_data() {
        let data = vec![0x10u8, 0x20];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        assert!(reader.read_chunk(3).is_err());
    }

    #[test]
    fn test_get_ref() {
        let data = vec![0x10u8, 0x20];
        let cursor = Cursor::new(data.clone());
        let reader = BufferedReader::new(cursor);

        assert_eq!(reader.get_ref().get_ref(), &data);
    }

    #[test]
    fn test_get_mut() {
        let data = vec![0x10u8, 0x20];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::new(cursor);

        reader.get_mut().set_position(1);

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 0x20);
    }

    #[test]
    fn test_into_inner() {
        let data = vec![0x10u8, 0x20];
        let cursor = Cursor::new(data.clone());
        let reader = BufferedReader::new(cursor);

        let inner = reader.into_inner();
        assert_eq!(inner.get_ref(), &data);
    }

    #[test]
    fn test_large_read_bypasses_buffer() {
        let data = vec![0u8; 16384];
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::with_capacity(1024, cursor);

        let mut buf = vec![0u8; 8192];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 8192);
    }

    #[test]
    fn test_sequential_small_reads() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);
        let mut reader = BufferedReader::with_capacity(32, cursor);

        for i in 0u8..100 {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf).unwrap();
            assert_eq!(buf[0], i);
        }
    }
}
