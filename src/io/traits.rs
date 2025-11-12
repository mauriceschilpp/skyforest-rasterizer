//! Core I/O traits

use std::io::{Read, Seek};

/// Trait for readers that support both reading and seeking operations
///
/// This trait combines [`Read`] and [`Seek`] to provide a unified interface
/// for file-based I/O operations. It is automatically implemented for any type
/// that implements both traits along with [`Send`] and [`Sync`].
pub trait SeekableReader: Read + Seek + Send + Sync {}

impl<T: Read + Seek + Send + Sync> SeekableReader for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    #[test]
    fn test_cursor_implements_seekable_reader() {
        let data = vec![1u8, 2, 3, 4];
        let cursor = Cursor::new(data);

        fn accepts_seekable<R: SeekableReader>(_r: R) {}
        accepts_seekable(cursor);
    }

    #[test]
    fn test_read_operations() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let mut reader: Box<dyn SeekableReader> = Box::new(Cursor::new(data));

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x10, 0x20]);

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x30, 0x40]);
    }

    #[test]
    fn test_seek_operations() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let mut reader: Box<dyn SeekableReader> = Box::new(Cursor::new(data));

        reader.seek(SeekFrom::Start(2)).unwrap();

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 0x30);
    }

    #[test]
    fn test_seek_from_end() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let mut reader: Box<dyn SeekableReader> = Box::new(Cursor::new(data));

        reader.seek(SeekFrom::End(-1)).unwrap();

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 0x40);
    }

    #[test]
    fn test_seek_from_current() {
        let data = vec![0x10u8, 0x20, 0x30, 0x40];
        let mut reader: Box<dyn SeekableReader> = Box::new(Cursor::new(data));

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 0x10);

        reader.seek(SeekFrom::Current(1)).unwrap();
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf[0], 0x30);
    }
}
