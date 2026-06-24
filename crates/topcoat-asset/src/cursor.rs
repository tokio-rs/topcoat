/// Const-friendly cursor for writing primitives into a fixed byte buffer.
pub struct ConstWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ConstWriter<'a> {
    pub const fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub const fn write_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            self.buf[self.pos] = bytes[i];
            self.pos += 1;
            i += 1;
        }
    }

    pub const fn write_u16_le(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub const fn write_u64_le(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    #[allow(clippy::cast_possible_truncation)]
    pub const fn write_str(&mut self, s: &str) {
        let len = s.len() as u16;
        self.write_u16_le(len);
        self.write_bytes(s.as_bytes());
    }

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

/// Const-friendly cursor for reading primitives out of a byte buffer.
///
/// Reads return slices borrowed from the underlying buffer, which keeps the
/// reader allocation-free; callers can `.to_owned()` if they need ownership.
pub struct ConstReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ConstReader<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

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

    pub const fn read_u16_le(&mut self) -> Option<u16> {
        let Some(bytes) = self.read_bytes(2) else {
            return None;
        };
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub const fn read_u64_le(&mut self) -> Option<u64> {
        let Some(bytes) = self.read_bytes(8) else {
            return None;
        };
        Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

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

    /// Reads an optional string, encoding three states: `None` when the buffer
    /// is exhausted, `Some(None)` for an absent string, and `Some(Some(_))` for
    /// a present string.
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

    pub const fn skip(&mut self, n: usize) -> Option<()> {
        match self.read_bytes(n) {
            Some(_) => Some(()),
            None => None,
        }
    }
}
