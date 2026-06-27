#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnicodeCodePoint(u32);

impl UnicodeCodePoint {
    #[must_use]
    pub const fn new(code_point: u32) -> Self {
        assert!(
            code_point <= 0x10_FFFF,
            "unicode code point exceeds U+10FFFF"
        );
        Self(code_point)
    }
}

impl From<UnicodeCodePoint> for u32 {
    fn from(value: UnicodeCodePoint) -> Self {
        value.0
    }
}

pub struct CodePointOutOfRangeError;

impl TryFrom<u32> for UnicodeCodePoint {
    type Error = CodePointOutOfRangeError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > 0x10_FFFF {
            return Err(CodePointOutOfRangeError);
        }
        Ok(Self(value))
    }
}

impl std::fmt::Display for UnicodeCodePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code_point = self.0;
        write!(f, "U+{code_point:04X}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnicodeRange {
    start: UnicodeCodePoint,
    end: UnicodeCodePoint,
}

impl UnicodeRange {
    #[must_use]
    pub const fn new(start: UnicodeCodePoint, end: UnicodeCodePoint) -> Self {
        assert!(end.0 >= start.0, "unicode range must not be empty");
        Self { start, end }
    }

    #[must_use]
    pub const fn start(&self) -> UnicodeCodePoint {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> UnicodeCodePoint {
        self.end
    }
}

impl std::fmt::Display for UnicodeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            self.start.fmt(f)
        } else {
            let start = self.start.0;
            let end = self.end.0;
            write!(f, "U+{start:04X}-{end:04X}")
        }
    }
}
