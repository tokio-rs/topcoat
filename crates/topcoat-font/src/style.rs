//! Font styles for building CSS `font-style` descriptors on `@font-face`
//! rules.

/// An oblique slant angle in degrees, in `-90.0..=90.0`.
///
/// This is the angle a glyph is slanted from upright, as used by
/// [`FontStyle::Oblique`]. CSS uses [`ObliqueAngle::DEFAULT`] (`14deg`) when an
/// oblique style omits its angle.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ObliqueAngle(f32);

impl ObliqueAngle {
    /// The angle CSS assumes for an oblique style with no explicit angle,
    /// `14deg`.
    pub const DEFAULT: Self = Self(14.0);

    /// Create an oblique angle from a value in degrees.
    ///
    /// # Panics
    ///
    /// Panics if `degrees` is outside `-90.0..=90.0`. Use
    /// `ObliqueAngle::try_from` for a non-panicking conversion.
    #[must_use]
    pub const fn new(degrees: f32) -> Self {
        assert!(
            degrees >= -90.0 && degrees <= 90.0,
            "oblique angle out of range -90deg..=90deg"
        );
        Self(degrees)
    }
}

impl Default for ObliqueAngle {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<ObliqueAngle> for f32 {
    fn from(value: ObliqueAngle) -> Self {
        value.0
    }
}

/// Error returned when converting an angle outside `-90.0..=90.0` degrees into
/// an [`ObliqueAngle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObliqueAngleOutOfRangeError;

impl std::fmt::Display for ObliqueAngleOutOfRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("oblique angle out of range -90deg..=90deg")
    }
}

impl std::error::Error for ObliqueAngleOutOfRangeError {}

impl TryFrom<f32> for ObliqueAngle {
    type Error = ObliqueAngleOutOfRangeError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !(-90.0..=90.0).contains(&value) {
            return Err(ObliqueAngleOutOfRangeError);
        }
        Ok(Self(value))
    }
}

impl std::fmt::Display for ObliqueAngle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}deg", self.0)
    }
}

/// An inclusive range of [`ObliqueAngle`]s, as carried by a variable font.
///
/// Displays as a single angle (`14deg`) when it covers one angle, or as the
/// space-separated pair CSS expects otherwise (`20deg 40deg`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObliqueAngleRange {
    start: ObliqueAngle,
    end: ObliqueAngle,
}

impl ObliqueAngleRange {
    /// Create an inclusive range from `start` to `end`.
    ///
    /// # Panics
    ///
    /// Panics if `end` is before `start`.
    #[must_use]
    pub const fn new(start: ObliqueAngle, end: ObliqueAngle) -> Self {
        assert!(end.0 >= start.0, "oblique angle range must not be empty");
        Self { start, end }
    }

    /// Create an inclusive range from two values in degrees.
    ///
    /// # Panics
    ///
    /// Panics if either value is outside `-90.0..=90.0`, or if `end` is before
    /// `start`.
    #[must_use]
    pub const fn from_degrees(start: f32, end: f32) -> Self {
        Self::new(ObliqueAngle::new(start), ObliqueAngle::new(end))
    }

    /// The smallest angle in the range.
    #[must_use]
    pub const fn start(&self) -> ObliqueAngle {
        self.start
    }

    /// The largest angle in the range, inclusive.
    #[must_use]
    pub const fn end(&self) -> ObliqueAngle {
        self.end
    }
}

impl std::fmt::Display for ObliqueAngleRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            self.start.fmt(f)
        } else {
            write!(f, "{} {}", self.start, self.end)
        }
    }
}

/// The style axis of a font face: upright, italic, or oblique.
///
/// Displays as a CSS `font-style` value: `normal`, `italic`, `oblique`, or
/// `oblique` followed by an angle or angle range (`oblique 14deg`,
/// `oblique 20deg 40deg`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStyle {
    /// Upright (`normal`) face.
    #[default]
    Normal,
    /// Italic (`italic`) face.
    Italic,
    /// Slanted (`oblique`) face, with an optional slant angle or angle range.
    ///
    /// `None` renders the bare `oblique` keyword, which CSS treats as
    /// [`ObliqueAngle::DEFAULT`].
    Oblique(Option<ObliqueAngleRange>),
}

impl FontStyle {
    /// An oblique face with no explicit angle (CSS `oblique`).
    #[must_use]
    pub const fn oblique() -> Self {
        Self::Oblique(None)
    }

    /// An oblique face slanted by a single angle in degrees
    /// (CSS `oblique 14deg`).
    ///
    /// # Panics
    ///
    /// Panics if `degrees` is outside `-90.0..=90.0`.
    #[must_use]
    pub const fn oblique_angle(degrees: f32) -> Self {
        let angle = ObliqueAngle::new(degrees);
        Self::Oblique(Some(ObliqueAngleRange::new(angle, angle)))
    }

    /// An oblique face spanning a range of angles in degrees, as carried by a
    /// variable font (CSS `oblique 20deg 40deg`).
    ///
    /// # Panics
    ///
    /// Panics if either value is outside `-90.0..=90.0`, or if `end` is before
    /// `start`.
    #[must_use]
    pub const fn oblique_range(start: f32, end: f32) -> Self {
        Self::Oblique(Some(ObliqueAngleRange::from_degrees(start, end)))
    }
}

impl std::fmt::Display for FontStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => f.write_str("normal"),
            Self::Italic => f.write_str("italic"),
            Self::Oblique(None) => f.write_str("oblique"),
            Self::Oblique(Some(angle)) => write!(f, "oblique {angle}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_and_italic_display_as_keywords() {
        assert_eq!(FontStyle::Normal.to_string(), "normal");
        assert_eq!(FontStyle::Italic.to_string(), "italic");
    }

    #[test]
    fn default_is_normal() {
        assert_eq!(FontStyle::default(), FontStyle::Normal);
    }

    #[test]
    fn bare_oblique_displays_without_an_angle() {
        assert_eq!(FontStyle::oblique().to_string(), "oblique");
    }

    #[test]
    fn oblique_with_an_angle_displays_the_angle() {
        assert_eq!(FontStyle::oblique_angle(14.0).to_string(), "oblique 14deg");
    }

    #[test]
    fn oblique_with_a_negative_angle_displays_the_sign() {
        assert_eq!(
            FontStyle::oblique_angle(-12.5).to_string(),
            "oblique -12.5deg",
        );
    }

    #[test]
    fn oblique_with_a_range_displays_both_angles() {
        assert_eq!(
            FontStyle::oblique_range(20.0, 40.0).to_string(),
            "oblique 20deg 40deg",
        );
    }

    #[test]
    fn oblique_range_collapses_when_start_equals_end() {
        assert_eq!(
            FontStyle::oblique_range(14.0, 14.0).to_string(),
            "oblique 14deg",
        );
    }

    #[test]
    fn default_oblique_angle_is_14_degrees() {
        assert_eq!(ObliqueAngle::DEFAULT.to_string(), "14deg");
        assert_eq!(ObliqueAngle::default(), ObliqueAngle::DEFAULT);
    }

    #[test]
    fn angle_converts_to_f32() {
        assert!((f32::from(ObliqueAngle::new(30.0)) - 30.0).abs() < 0.001);
    }

    #[test]
    fn try_from_accepts_the_bounds() {
        assert_eq!(ObliqueAngle::try_from(-90.0), Ok(ObliqueAngle::new(-90.0)));
        assert_eq!(ObliqueAngle::try_from(90.0), Ok(ObliqueAngle::new(90.0)));
    }

    #[test]
    fn try_from_rejects_out_of_range() {
        assert_eq!(
            ObliqueAngle::try_from(90.1),
            Err(ObliqueAngleOutOfRangeError),
        );
        assert_eq!(
            ObliqueAngle::try_from(-90.1),
            Err(ObliqueAngleOutOfRangeError),
        );
    }

    #[test]
    #[should_panic = "out of range"]
    fn new_panics_above_the_maximum() {
        let _ = ObliqueAngle::new(90.1);
    }

    #[test]
    #[should_panic = "out of range"]
    fn new_panics_below_the_minimum() {
        let _ = ObliqueAngle::new(-90.1);
    }

    #[test]
    #[should_panic = "empty"]
    fn range_panics_when_end_precedes_start() {
        let _ = ObliqueAngleRange::from_degrees(40.0, 20.0);
    }

    #[test]
    fn range_exposes_its_bounds() {
        let range = ObliqueAngleRange::from_degrees(20.0, 40.0);
        assert_eq!(range.start(), ObliqueAngle::new(20.0));
        assert_eq!(range.end(), ObliqueAngle::new(40.0));
    }
}
