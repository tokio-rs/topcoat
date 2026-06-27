//! Font weights and weight ranges for building CSS `font-weight` descriptors
//! on `@font-face` rules.

/// A font weight: an integer in `100..=900`.
///
/// These are the standard CSS `font-weight` values, from `100` (Thin) to `900`
/// (Black), in steps of `100`. Intermediate values are also permitted for
/// variable fonts. [`FontWeight::default`] is `400` (Normal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(u16);

impl FontWeight {
    /// Thin (Hairline), `100`.
    pub const THIN: Self = Self(100);
    /// Extra Light (Ultra Light), `200`.
    pub const EXTRA_LIGHT: Self = Self(200);
    /// Light, `300`.
    pub const LIGHT: Self = Self(300);
    /// Normal (Regular), `400`.
    pub const NORMAL: Self = Self(400);
    /// Medium, `500`.
    pub const MEDIUM: Self = Self(500);
    /// Semi Bold (Demi Bold), `600`.
    pub const SEMI_BOLD: Self = Self(600);
    /// Bold, `700`.
    pub const BOLD: Self = Self(700);
    /// Extra Bold (Ultra Bold), `800`.
    pub const EXTRA_BOLD: Self = Self(800);
    /// Black (Heavy), `900`.
    pub const BLACK: Self = Self(900);

    /// Create a font weight from a raw `u16`.
    ///
    /// # Panics
    ///
    /// Panics if `weight` is outside `100..=900`. Use
    /// `FontWeight::try_from` for a non-panicking conversion.
    #[must_use]
    pub const fn new(weight: u16) -> Self {
        assert!(
            weight >= 100 && weight <= 900,
            "font weight out of range 100..=900"
        );
        Self(weight)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl From<FontWeight> for u16 {
    fn from(value: FontWeight) -> Self {
        value.0
    }
}

/// Error returned when converting a `u16` outside `100..=900` into a
/// [`FontWeight`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontWeightOutOfRangeError;

impl std::fmt::Display for FontWeightOutOfRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("font weight out of range 100..=900")
    }
}

impl std::error::Error for FontWeightOutOfRangeError {}

impl TryFrom<u16> for FontWeight {
    type Error = FontWeightOutOfRangeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value < 100 || value > 900 {
            return Err(FontWeightOutOfRangeError);
        }
        Ok(Self(value))
    }
}

impl std::fmt::Display for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// An inclusive range of [`FontWeight`]s, as carried by a variable font.
///
/// Displays as a single CSS `font-weight` descriptor value: `400` when it
/// covers one weight, or `400 700` otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeightRange {
    start: FontWeight,
    end: FontWeight,
}

impl FontWeightRange {
    /// Create an inclusive range from `start` to `end`.
    ///
    /// # Panics
    ///
    /// Panics if `end` is before `start`.
    #[must_use]
    pub const fn new(start: FontWeight, end: FontWeight) -> Self {
        assert!(end.0 >= start.0, "font weight range must not be empty");
        Self { start, end }
    }

    /// Create an inclusive range from two raw weight values.
    ///
    /// # Panics
    ///
    /// Panics if either value is outside `100..=900`, or if `end` is before
    /// `start`.
    #[must_use]
    pub const fn from_u16(start: u16, end: u16) -> Self {
        Self::new(FontWeight::new(start), FontWeight::new(end))
    }

    /// The lightest weight in the range.
    #[must_use]
    pub const fn start(&self) -> FontWeight {
        self.start
    }

    /// The heaviest weight in the range, inclusive.
    #[must_use]
    pub const fn end(&self) -> FontWeight {
        self.end
    }
}

impl std::fmt::Display for FontWeightRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            self.start.fmt(f)
        } else {
            write!(f, "{} {}", self.start, self.end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(value: u16) -> FontWeight {
        FontWeight::new(value)
    }

    #[test]
    fn weight_displays_as_its_number() {
        assert_eq!(w(100).to_string(), "100");
        assert_eq!(w(900).to_string(), "900");
    }

    #[test]
    fn weight_converts_to_u16() {
        assert_eq!(u16::from(w(700)), 700);
    }

    #[test]
    fn default_is_normal() {
        assert_eq!(FontWeight::default(), FontWeight::NORMAL);
    }

    #[test]
    fn named_constants_match_their_values() {
        assert_eq!(FontWeight::THIN, w(100));
        assert_eq!(FontWeight::EXTRA_LIGHT, w(200));
        assert_eq!(FontWeight::LIGHT, w(300));
        assert_eq!(FontWeight::NORMAL, w(400));
        assert_eq!(FontWeight::MEDIUM, w(500));
        assert_eq!(FontWeight::SEMI_BOLD, w(600));
        assert_eq!(FontWeight::BOLD, w(700));
        assert_eq!(FontWeight::EXTRA_BOLD, w(800));
        assert_eq!(FontWeight::BLACK, w(900));
    }

    #[test]
    fn try_from_accepts_the_bounds() {
        assert_eq!(FontWeight::try_from(100), Ok(w(100)));
        assert_eq!(FontWeight::try_from(900), Ok(w(900)));
    }

    #[test]
    fn try_from_rejects_out_of_range() {
        assert_eq!(FontWeight::try_from(99), Err(FontWeightOutOfRangeError));
        assert_eq!(FontWeight::try_from(901), Err(FontWeightOutOfRangeError));
    }

    #[test]
    #[should_panic = "out of range"]
    fn new_panics_below_the_minimum() {
        let _ = FontWeight::new(99);
    }

    #[test]
    #[should_panic = "out of range"]
    fn new_panics_above_the_maximum() {
        let _ = FontWeight::new(901);
    }

    #[test]
    fn single_weight_range_displays_one_number() {
        assert_eq!(FontWeightRange::new(w(400), w(400)).to_string(), "400");
    }

    #[test]
    fn multi_weight_range_displays_both_numbers() {
        assert_eq!(FontWeightRange::new(w(400), w(700)).to_string(), "400 700");
    }

    #[test]
    #[should_panic = "empty"]
    fn range_panics_when_end_precedes_start() {
        let _ = FontWeightRange::new(w(700), w(400));
    }

    #[test]
    fn range_from_u16_matches_new() {
        assert_eq!(
            FontWeightRange::from_u16(400, 700),
            FontWeightRange::new(w(400), w(700)),
        );
    }

    #[test]
    #[should_panic = "out of range"]
    fn range_from_u16_panics_on_out_of_range() {
        let _ = FontWeightRange::from_u16(100, 901);
    }
}
