use std::fmt::{self, Display};

use topcoat_core::context::Cx;

use crate::{AttributeValueViewParts, PartsWriter};

/// A CSS length unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    // Absolute lengths.
    /// CSS pixels (`px`), 1/96th of an inch.
    Px,
    /// Centimeters (`cm`).
    Cm,
    /// Millimeters (`mm`), 1/10th of a centimeter.
    Mm,
    /// Quarter-millimeters (`Q`), 1/40th of a centimeter.
    Q,
    /// Inches (`in`), 2.54 centimeters.
    In,
    /// Picas (`pc`), 1/6th of an inch.
    Pc,
    /// Points (`pt`), 1/72nd of an inch.
    Pt,

    // Font-relative lengths.
    /// The element's font size (`em`).
    Em,
    /// The root element's font size (`rem`).
    Rem,
    /// The x-height of the element's font (`ex`).
    Ex,
    /// The x-height of the root element's font (`rex`).
    Rex,
    /// The cap height of the element's font (`cap`).
    Cap,
    /// The cap height of the root element's font (`rcap`).
    Rcap,
    /// The advance of the `0` glyph in the element's font (`ch`).
    Ch,
    /// The advance of the `0` glyph in the root element's font (`rch`).
    Rch,
    /// The advance of a fullwidth glyph in the element's font (`ic`).
    Ic,
    /// The advance of a fullwidth glyph in the root element's font (`ric`).
    Ric,
    /// The line height of the element (`lh`).
    Lh,
    /// The line height of the root element (`rlh`).
    Rlh,

    // Viewport-percentage lengths.
    /// 1% of the viewport's width (`vw`).
    Vw,
    /// 1% of the viewport's height (`vh`).
    Vh,
    /// 1% of the viewport's inline-axis size (`vi`).
    Vi,
    /// 1% of the viewport's block-axis size (`vb`).
    Vb,
    /// 1% of the viewport's smaller dimension (`vmin`).
    Vmin,
    /// 1% of the viewport's larger dimension (`vmax`).
    Vmax,

    // Small-viewport-percentage lengths.
    /// 1% of the small viewport's width (`svw`).
    Svw,
    /// 1% of the small viewport's height (`svh`).
    Svh,
    /// 1% of the small viewport's inline-axis size (`svi`).
    Svi,
    /// 1% of the small viewport's block-axis size (`svb`).
    Svb,
    /// 1% of the small viewport's smaller dimension (`svmin`).
    Svmin,
    /// 1% of the small viewport's larger dimension (`svmax`).
    Svmax,

    // Large-viewport-percentage lengths.
    /// 1% of the large viewport's width (`lvw`).
    Lvw,
    /// 1% of the large viewport's height (`lvh`).
    Lvh,
    /// 1% of the large viewport's inline-axis size (`lvi`).
    Lvi,
    /// 1% of the large viewport's block-axis size (`lvb`).
    Lvb,
    /// 1% of the large viewport's smaller dimension (`lvmin`).
    Lvmin,
    /// 1% of the large viewport's larger dimension (`lvmax`).
    Lvmax,

    // Dynamic-viewport-percentage lengths.
    /// 1% of the dynamic viewport's width (`dvw`).
    Dvw,
    /// 1% of the dynamic viewport's height (`dvh`).
    Dvh,
    /// 1% of the dynamic viewport's inline-axis size (`dvi`).
    Dvi,
    /// 1% of the dynamic viewport's block-axis size (`dvb`).
    Dvb,
    /// 1% of the dynamic viewport's smaller dimension (`dvmin`).
    Dvmin,
    /// 1% of the dynamic viewport's larger dimension (`dvmax`).
    Dvmax,

    // Container-query lengths.
    /// 1% of the query container's width (`cqw`).
    Cqw,
    /// 1% of the query container's height (`cqh`).
    Cqh,
    /// 1% of the query container's inline size (`cqi`).
    Cqi,
    /// 1% of the query container's block size (`cqb`).
    Cqb,
    /// 1% of the query container's smaller dimension (`cqmin`).
    Cqmin,
    /// 1% of the query container's larger dimension (`cqmax`).
    Cqmax,
}

impl LengthUnit {
    /// Returns the CSS suffix for this unit, such as `"px"` or `"rem"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Px => "px",
            Self::Cm => "cm",
            Self::Mm => "mm",
            Self::Q => "Q",
            Self::In => "in",
            Self::Pc => "pc",
            Self::Pt => "pt",
            Self::Em => "em",
            Self::Rem => "rem",
            Self::Ex => "ex",
            Self::Rex => "rex",
            Self::Cap => "cap",
            Self::Rcap => "rcap",
            Self::Ch => "ch",
            Self::Rch => "rch",
            Self::Ic => "ic",
            Self::Ric => "ric",
            Self::Lh => "lh",
            Self::Rlh => "rlh",
            Self::Vw => "vw",
            Self::Vh => "vh",
            Self::Vi => "vi",
            Self::Vb => "vb",
            Self::Vmin => "vmin",
            Self::Vmax => "vmax",
            Self::Svw => "svw",
            Self::Svh => "svh",
            Self::Svi => "svi",
            Self::Svb => "svb",
            Self::Svmin => "svmin",
            Self::Svmax => "svmax",
            Self::Lvw => "lvw",
            Self::Lvh => "lvh",
            Self::Lvi => "lvi",
            Self::Lvb => "lvb",
            Self::Lvmin => "lvmin",
            Self::Lvmax => "lvmax",
            Self::Dvw => "dvw",
            Self::Dvh => "dvh",
            Self::Dvi => "dvi",
            Self::Dvb => "dvb",
            Self::Dvmin => "dvmin",
            Self::Dvmax => "dvmax",
            Self::Cqw => "cqw",
            Self::Cqh => "cqh",
            Self::Cqi => "cqi",
            Self::Cqb => "cqb",
            Self::Cqmin => "cqmin",
            Self::Cqmax => "cqmax",
        }
    }
}

impl Display for LengthUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A CSS length: a numeric value paired with a [`LengthUnit`].
///
/// Construct one with a per-unit helper like [`Length::px`] or [`Length::rem`],
/// or from a value and unit with [`Length::new`]. Plain numbers convert to a
/// pixel length, so a `#[into]` component parameter accepts `size: 24` as a
/// 24-pixel length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    value: f32,
    unit: LengthUnit,
}

impl Length {
    /// Creates a length from a numeric value and a unit.
    #[must_use]
    pub const fn new(value: f32, unit: LengthUnit) -> Self {
        Self { value, unit }
    }

    /// Returns the numeric value of this length.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.value
    }

    /// Returns the unit of this length.
    #[must_use]
    pub const fn unit(self) -> LengthUnit {
        self.unit
    }

    /// A length in `px` units.
    #[must_use]
    pub const fn px(value: f32) -> Self {
        Self::new(value, LengthUnit::Px)
    }

    /// A length in `cm` units.
    #[must_use]
    pub const fn cm(value: f32) -> Self {
        Self::new(value, LengthUnit::Cm)
    }

    /// A length in `mm` units.
    #[must_use]
    pub const fn mm(value: f32) -> Self {
        Self::new(value, LengthUnit::Mm)
    }

    /// A length in `Q` units.
    #[must_use]
    pub const fn q(value: f32) -> Self {
        Self::new(value, LengthUnit::Q)
    }

    /// A length in `in` units.
    #[must_use]
    pub const fn r#in(value: f32) -> Self {
        Self::new(value, LengthUnit::In)
    }

    /// A length in `pc` units.
    #[must_use]
    pub const fn pc(value: f32) -> Self {
        Self::new(value, LengthUnit::Pc)
    }

    /// A length in `pt` units.
    #[must_use]
    pub const fn pt(value: f32) -> Self {
        Self::new(value, LengthUnit::Pt)
    }

    /// A length in `em` units.
    #[must_use]
    pub const fn em(value: f32) -> Self {
        Self::new(value, LengthUnit::Em)
    }

    /// A length in `rem` units.
    #[must_use]
    pub const fn rem(value: f32) -> Self {
        Self::new(value, LengthUnit::Rem)
    }

    /// A length in `ex` units.
    #[must_use]
    pub const fn ex(value: f32) -> Self {
        Self::new(value, LengthUnit::Ex)
    }

    /// A length in `rex` units.
    #[must_use]
    pub const fn rex(value: f32) -> Self {
        Self::new(value, LengthUnit::Rex)
    }

    /// A length in `cap` units.
    #[must_use]
    pub const fn cap(value: f32) -> Self {
        Self::new(value, LengthUnit::Cap)
    }

    /// A length in `rcap` units.
    #[must_use]
    pub const fn rcap(value: f32) -> Self {
        Self::new(value, LengthUnit::Rcap)
    }

    /// A length in `ch` units.
    #[must_use]
    pub const fn ch(value: f32) -> Self {
        Self::new(value, LengthUnit::Ch)
    }

    /// A length in `rch` units.
    #[must_use]
    pub const fn rch(value: f32) -> Self {
        Self::new(value, LengthUnit::Rch)
    }

    /// A length in `ic` units.
    #[must_use]
    pub const fn ic(value: f32) -> Self {
        Self::new(value, LengthUnit::Ic)
    }

    /// A length in `ric` units.
    #[must_use]
    pub const fn ric(value: f32) -> Self {
        Self::new(value, LengthUnit::Ric)
    }

    /// A length in `lh` units.
    #[must_use]
    pub const fn lh(value: f32) -> Self {
        Self::new(value, LengthUnit::Lh)
    }

    /// A length in `rlh` units.
    #[must_use]
    pub const fn rlh(value: f32) -> Self {
        Self::new(value, LengthUnit::Rlh)
    }

    /// A length in `vw` units.
    #[must_use]
    pub const fn vw(value: f32) -> Self {
        Self::new(value, LengthUnit::Vw)
    }

    /// A length in `vh` units.
    #[must_use]
    pub const fn vh(value: f32) -> Self {
        Self::new(value, LengthUnit::Vh)
    }

    /// A length in `vi` units.
    #[must_use]
    pub const fn vi(value: f32) -> Self {
        Self::new(value, LengthUnit::Vi)
    }

    /// A length in `vb` units.
    #[must_use]
    pub const fn vb(value: f32) -> Self {
        Self::new(value, LengthUnit::Vb)
    }

    /// A length in `vmin` units.
    #[must_use]
    pub const fn vmin(value: f32) -> Self {
        Self::new(value, LengthUnit::Vmin)
    }

    /// A length in `vmax` units.
    #[must_use]
    pub const fn vmax(value: f32) -> Self {
        Self::new(value, LengthUnit::Vmax)
    }

    /// A length in `svw` units.
    #[must_use]
    pub const fn svw(value: f32) -> Self {
        Self::new(value, LengthUnit::Svw)
    }

    /// A length in `svh` units.
    #[must_use]
    pub const fn svh(value: f32) -> Self {
        Self::new(value, LengthUnit::Svh)
    }

    /// A length in `svi` units.
    #[must_use]
    pub const fn svi(value: f32) -> Self {
        Self::new(value, LengthUnit::Svi)
    }

    /// A length in `svb` units.
    #[must_use]
    pub const fn svb(value: f32) -> Self {
        Self::new(value, LengthUnit::Svb)
    }

    /// A length in `svmin` units.
    #[must_use]
    pub const fn svmin(value: f32) -> Self {
        Self::new(value, LengthUnit::Svmin)
    }

    /// A length in `svmax` units.
    #[must_use]
    pub const fn svmax(value: f32) -> Self {
        Self::new(value, LengthUnit::Svmax)
    }

    /// A length in `lvw` units.
    #[must_use]
    pub const fn lvw(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvw)
    }

    /// A length in `lvh` units.
    #[must_use]
    pub const fn lvh(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvh)
    }

    /// A length in `lvi` units.
    #[must_use]
    pub const fn lvi(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvi)
    }

    /// A length in `lvb` units.
    #[must_use]
    pub const fn lvb(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvb)
    }

    /// A length in `lvmin` units.
    #[must_use]
    pub const fn lvmin(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvmin)
    }

    /// A length in `lvmax` units.
    #[must_use]
    pub const fn lvmax(value: f32) -> Self {
        Self::new(value, LengthUnit::Lvmax)
    }

    /// A length in `dvw` units.
    #[must_use]
    pub const fn dvw(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvw)
    }

    /// A length in `dvh` units.
    #[must_use]
    pub const fn dvh(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvh)
    }

    /// A length in `dvi` units.
    #[must_use]
    pub const fn dvi(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvi)
    }

    /// A length in `dvb` units.
    #[must_use]
    pub const fn dvb(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvb)
    }

    /// A length in `dvmin` units.
    #[must_use]
    pub const fn dvmin(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvmin)
    }

    /// A length in `dvmax` units.
    #[must_use]
    pub const fn dvmax(value: f32) -> Self {
        Self::new(value, LengthUnit::Dvmax)
    }

    /// A length in `cqw` units.
    #[must_use]
    pub const fn cqw(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqw)
    }

    /// A length in `cqh` units.
    #[must_use]
    pub const fn cqh(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqh)
    }

    /// A length in `cqi` units.
    #[must_use]
    pub const fn cqi(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqi)
    }

    /// A length in `cqb` units.
    #[must_use]
    pub const fn cqb(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqb)
    }

    /// A length in `cqmin` units.
    #[must_use]
    pub const fn cqmin(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqmin)
    }

    /// A length in `cqmax` units.
    #[must_use]
    pub const fn cqmax(value: f32) -> Self {
        Self::new(value, LengthUnit::Cqmax)
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = zmij::Buffer::new();
        f.write_str(buffer.format(self.value))?;
        f.write_str(self.unit.as_str())?;
        Ok(())
    }
}

impl From<u16> for Length {
    fn from(value: u16) -> Self {
        Self::px(f32::from(value))
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::px(value)
    }
}

impl AttributeValueViewParts for Length {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_f32(self.value);
        parts.push_str_unescaped(self.unit.as_str());
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{HtmlContext, View, ViewParts};

    /// Every unit constructor paired with its rendered form. The numeric value
    /// is the same across cases so each assertion focuses on the unit suffix.
    const UNITS: &[(Length, &str)] = &[
        (Length::px(2.0), "2.0px"),
        (Length::cm(2.0), "2.0cm"),
        (Length::mm(2.0), "2.0mm"),
        (Length::q(2.0), "2.0Q"),
        (Length::r#in(2.0), "2.0in"),
        (Length::pc(2.0), "2.0pc"),
        (Length::pt(2.0), "2.0pt"),
        (Length::em(2.0), "2.0em"),
        (Length::rem(2.0), "2.0rem"),
        (Length::ex(2.0), "2.0ex"),
        (Length::rex(2.0), "2.0rex"),
        (Length::cap(2.0), "2.0cap"),
        (Length::rcap(2.0), "2.0rcap"),
        (Length::ch(2.0), "2.0ch"),
        (Length::rch(2.0), "2.0rch"),
        (Length::ic(2.0), "2.0ic"),
        (Length::ric(2.0), "2.0ric"),
        (Length::lh(2.0), "2.0lh"),
        (Length::rlh(2.0), "2.0rlh"),
        (Length::vw(2.0), "2.0vw"),
        (Length::vh(2.0), "2.0vh"),
        (Length::vi(2.0), "2.0vi"),
        (Length::vb(2.0), "2.0vb"),
        (Length::vmin(2.0), "2.0vmin"),
        (Length::vmax(2.0), "2.0vmax"),
        (Length::svw(2.0), "2.0svw"),
        (Length::svh(2.0), "2.0svh"),
        (Length::svi(2.0), "2.0svi"),
        (Length::svb(2.0), "2.0svb"),
        (Length::svmin(2.0), "2.0svmin"),
        (Length::svmax(2.0), "2.0svmax"),
        (Length::lvw(2.0), "2.0lvw"),
        (Length::lvh(2.0), "2.0lvh"),
        (Length::lvi(2.0), "2.0lvi"),
        (Length::lvb(2.0), "2.0lvb"),
        (Length::lvmin(2.0), "2.0lvmin"),
        (Length::lvmax(2.0), "2.0lvmax"),
        (Length::dvw(2.0), "2.0dvw"),
        (Length::dvh(2.0), "2.0dvh"),
        (Length::dvi(2.0), "2.0dvi"),
        (Length::dvb(2.0), "2.0dvb"),
        (Length::dvmin(2.0), "2.0dvmin"),
        (Length::dvmax(2.0), "2.0dvmax"),
        (Length::cqw(2.0), "2.0cqw"),
        (Length::cqh(2.0), "2.0cqh"),
        (Length::cqi(2.0), "2.0cqi"),
        (Length::cqb(2.0), "2.0cqb"),
        (Length::cqmin(2.0), "2.0cqmin"),
        (Length::cqmax(2.0), "2.0cqmax"),
    ];

    fn render(value: impl AttributeValueViewParts) -> String {
        let mut parts = ViewParts::new();
        value.into_view_parts(
            &Cx::default(),
            &mut PartsWriter::new(&mut parts, HtmlContext::AttributeValue),
        );
        View::new(parts).render(&Cx::default())
    }

    #[test]
    fn displays_with_unit() {
        for &(length, expected) in UNITS {
            assert_eq!(length.to_string(), expected, "Display for {length:?}");
        }
    }

    #[test]
    fn renders_view_parts_with_unit() {
        for &(length, expected) in UNITS {
            assert_eq!(render(length), expected, "view parts for {length:?}");
        }
    }

    #[test]
    fn unit_reports_its_suffix() {
        assert_eq!(LengthUnit::Px.as_str(), "px");
        assert_eq!(LengthUnit::Q.as_str(), "Q");
        assert_eq!(LengthUnit::In.as_str(), "in");
        assert_eq!(LengthUnit::Cqmax.as_str(), "cqmax");
        assert_eq!(LengthUnit::Rem.to_string(), "rem");
    }

    #[test]
    fn getters_expose_value_and_unit() {
        let length = Length::rem(1.5);
        assert_eq!(length.unit(), LengthUnit::Rem);
        // Rebuilding from the getters yields an equal length, which exercises
        // `value` without comparing floats directly.
        assert_eq!(Length::new(length.value(), length.unit()), length);
    }

    #[test]
    fn formats_fractional_and_negative_values() {
        assert_eq!(Length::px(1.5).to_string(), "1.5px");
        assert_eq!(render(Length::px(1.5)), "1.5px");
        assert_eq!(Length::em(-0.5).to_string(), "-0.5em");
        assert_eq!(render(Length::em(-0.5)), "-0.5em");
        assert_eq!(Length::rem(0.0).to_string(), "0.0rem");
        assert_eq!(render(Length::rem(0.0)), "0.0rem");
    }

    #[test]
    fn numbers_convert_to_pixels() {
        assert_eq!(Length::from(24u16), Length::px(24.0));
        assert_eq!(Length::from(1.5f32), Length::px(1.5));
    }

    #[test]
    fn attribute_is_always_present() {
        assert!(Length::px(0.0).attribute_present());
        assert!(Length::cqmax(1.0).attribute_present());
    }
}
