//! Cursors for writing and reading a simple binary encoding in `const`
//! contexts.
//!
//! [`ConstWriter`] writes values into a byte buffer and [`ConstReader`] reads
//! them back in the same order. Integers are little-endian, and strings are
//! prefixed with their length as a `u16`.

/// A cursor that writes values into a fixed byte buffer, usable in `const`
/// contexts.
///
/// Every write panics if the buffer is too small for it.
///
/// ```
/// use topcoat_core::cursor::{ConstReader, ConstWriter};
///
/// let mut buf = [0; 16];
/// let mut writer = ConstWriter::new(&mut buf);
/// writer.write_u16_le(7);
/// writer.write_str("hi");
///
/// let mut reader = ConstReader::new(&buf);
/// assert_eq!(reader.read_u16_le(), Some(7));
/// assert_eq!(reader.read_str(), Some("hi"));
/// ```
pub struct ConstWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ConstWriter<'a> {
    /// Creates a writer that starts at the beginning of `buf`.
    pub const fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Writes `bytes` as they are.
    pub const fn write_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            self.buf[self.pos] = bytes[i];
            self.pos += 1;
            i += 1;
        }
    }

    /// Writes `v` as two little-endian bytes.
    pub const fn write_u16_le(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Writes `v` as eight little-endian bytes.
    pub const fn write_u64_le(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Writes `s` prefixed with its length in bytes as a `u16`.
    ///
    /// The string must be shorter than 65536 bytes. A longer string writes a
    /// truncated length that [`ConstReader::read_str`] cannot read back.
    #[allow(clippy::cast_possible_truncation)]
    pub const fn write_str(&mut self, s: &str) {
        let len = s.len() as u16;
        self.write_u16_le(len);
        self.write_bytes(s.as_bytes());
    }

    /// Writes an optional string: a `0` byte for `None`, or a `1` byte
    /// followed by the string as [`write_str`](Self::write_str) writes it.
    pub const fn write_str_opt(&mut self, s: Option<&str>) {
        match s {
            Some(s) => {
                self.write_bytes(&[1]);
                self.write_str(s);
            }
            None => self.write_bytes(&[0]),
        }
    }
}

/// A cursor that reads values out of a byte buffer written by a
/// [`ConstWriter`], usable in `const` contexts.
///
/// Every read returns `None` if the buffer ends too early or holds invalid
/// data. Reads borrow from the buffer instead of allocating.
pub struct ConstReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ConstReader<'a> {
    /// Creates a reader that starts at the beginning of `buf`.
    #[must_use]
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Reads the next `n` bytes.
    pub const fn read_bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let Some((_, rest)) = self.buf.split_at_checked(self.pos) else {
            return None;
        };
        let Some((head, _)) = rest.split_at_checked(n) else {
            return None;
        };
        self.pos += n;
        Some(head)
    }

    /// Reads a little-endian `u16`.
    pub const fn read_u16_le(&mut self) -> Option<u16> {
        let Some(bytes) = self.read_bytes(2) else {
            return None;
        };
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a little-endian `u64`.
    pub const fn read_u64_le(&mut self) -> Option<u64> {
        let Some(bytes) = self.read_bytes(8) else {
            return None;
        };
        Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Reads a string written by [`ConstWriter::write_str`].
    ///
    /// Returns `None` if the bytes are not valid UTF-8.
    pub const fn read_str(&mut self) -> Option<&'a str> {
        let Some(len) = self.read_u16_le() else {
            return None;
        };
        let Some(bytes) = self.read_bytes(len as usize) else {
            return None;
        };
        match std::str::from_utf8(bytes) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    /// Reads an optional string written by [`ConstWriter::write_str_opt`].
    ///
    /// Returns `Some(None)` for an absent string, `Some(Some(_))` for a
    /// present one, and `None` if the data is missing or invalid.
    #[allow(clippy::option_option)]
    pub const fn read_str_opt(&mut self) -> Option<Option<&'a str>> {
        let Some(tag) = self.read_bytes(1) else {
            return None;
        };
        match tag[0] {
            0 => Some(None),
            1 => match self.read_str() {
                Some(s) => Some(Some(s)),
                None => None,
            },
            _ => None,
        }
    }

    /// Skips the next `n` bytes, returning `None` if fewer remain.
    pub const fn skip(&mut self, n: usize) -> Option<()> {
        match self.read_bytes(n) {
            Some(_) => Some(()),
            None => None,
        }
    }
}
