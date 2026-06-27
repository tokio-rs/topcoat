//! Unicode code points and ranges for building CSS `unicode-range`
//! descriptors on subsetted `@font-face` rules.

use std::ops::Deref;

/// A Unicode code point: an integer in `U+0000..=U+10FFFF`.
///
/// The upper bound is the Unicode code space, which is intentionally broader
/// than [`char`]: surrogate code points (`U+D800..=U+DFFF`) are not valid
/// [`char`]s, but are valid in a CSS `unicode-range`, which addresses code
/// points rather than scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnicodeCodePoint(u32);

impl UnicodeCodePoint {
    /// Create a code point from a raw `u32`.
    ///
    /// # Panics
    ///
    /// Panics if `code_point` is greater than `U+10FFFF`. Use
    /// `UnicodeCodePoint::try_from` for a non-panicking conversion.
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

/// Error returned when converting a `u32` greater than `U+10FFFF` into a
/// [`UnicodeCodePoint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodePointOutOfRangeError;

impl std::fmt::Display for CodePointOutOfRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("code point exceeds U+10FFFF")
    }
}

impl std::error::Error for CodePointOutOfRangeError {}

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

/// An inclusive range of [`UnicodeCodePoint`]s.
///
/// Displays as a single CSS `unicode-range` interval: `U+0041` when it covers
/// one code point, or `U+0041-005A` otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnicodeRange {
    start: UnicodeCodePoint,
    end: UnicodeCodePoint,
}

impl UnicodeRange {
    /// Create an inclusive range from `start` to `end`.
    ///
    /// # Panics
    ///
    /// Panics if `end` is before `start`.
    #[must_use]
    pub const fn new(start: UnicodeCodePoint, end: UnicodeCodePoint) -> Self {
        assert!(end.0 >= start.0, "unicode range must not be empty");
        Self { start, end }
    }

    /// The first code point in the range.
    #[must_use]
    pub const fn start(&self) -> UnicodeCodePoint {
        self.start
    }

    /// The last code point in the range, inclusive.
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

/// A set of [`UnicodeRange`]s, the value of a CSS `unicode-range` descriptor.
///
/// Displays as the comma-separated list CSS expects, e.g.
/// `U+0000-00FF, U+0131, U+0152-0153`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnicodeRanges(&'static [UnicodeRange]);

impl UnicodeRanges {
    /// Wrap a slice of ranges.
    #[must_use]
    pub const fn new(ranges: &'static [UnicodeRange]) -> Self {
        Self(ranges)
    }
}

impl std::fmt::Display for UnicodeRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, range) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            range.fmt(f)?;
        }
        Ok(())
    }
}

impl Deref for UnicodeRanges {
    type Target = [UnicodeRange];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cp(value: u32) -> UnicodeCodePoint {
        UnicodeCodePoint::new(value)
    }

    #[test]
    fn code_point_displays_as_padded_hex() {
        assert_eq!(cp(0x41).to_string(), "U+0041");
        assert_eq!(cp(0x10_FFFF).to_string(), "U+10FFFF");
    }

    #[test]
    fn code_point_converts_to_u32() {
        assert_eq!(u32::from(cp(0x1F600)), 0x1F600);
    }

    #[test]
    fn try_from_accepts_surrogates_and_the_maximum() {
        assert_eq!(UnicodeCodePoint::try_from(0xD800), Ok(cp(0xD800)));
        assert_eq!(UnicodeCodePoint::try_from(0x10_FFFF), Ok(cp(0x10_FFFF)));
    }

    #[test]
    fn try_from_rejects_out_of_range() {
        assert_eq!(
            UnicodeCodePoint::try_from(0x11_0000),
            Err(CodePointOutOfRangeError),
        );
    }

    #[test]
    #[should_panic = "exceeds"]
    fn new_panics_on_out_of_range_code_point() {
        let _ = UnicodeCodePoint::new(0x11_0000);
    }

    #[test]
    fn single_code_point_range_omits_the_dash() {
        assert_eq!(UnicodeRange::new(cp(0x41), cp(0x41)).to_string(), "U+0041");
    }

    #[test]
    fn multi_code_point_range_includes_the_dash() {
        assert_eq!(
            UnicodeRange::new(cp(0x41), cp(0x5A)).to_string(),
            "U+0041-005A",
        );
    }

    #[test]
    #[should_panic = "empty"]
    fn range_panics_when_end_precedes_start() {
        let _ = UnicodeRange::new(cp(0x5A), cp(0x41));
    }

    #[test]
    fn ranges_display_comma_separated() {
        const RANGES: UnicodeRanges = UnicodeRanges::new(&[
            UnicodeRange::new(UnicodeCodePoint::new(0x00), UnicodeCodePoint::new(0xFF)),
            UnicodeRange::new(UnicodeCodePoint::new(0x131), UnicodeCodePoint::new(0x131)),
            UnicodeRange::new(UnicodeCodePoint::new(0x152), UnicodeCodePoint::new(0x153)),
        ]);
        assert_eq!(RANGES.to_string(), "U+0000-00FF, U+0131, U+0152-0153");
    }

    #[test]
    fn empty_ranges_display_as_empty_string() {
        const RANGES: UnicodeRanges = UnicodeRanges::new(&[]);
        assert_eq!(RANGES.to_string(), "");
    }

    #[test]
    fn ranges_deref_to_their_slice() {
        const RANGES: UnicodeRanges =
            UnicodeRanges::new(&[UnicodeRange::new(UnicodeCodePoint::new(0x00), UnicodeCodePoint::new(0xFF))]);
        assert_eq!(RANGES.len(), 1);
        assert_eq!(RANGES[0].start(), cp(0x00));
    }
}
