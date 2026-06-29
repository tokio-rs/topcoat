//! The const [`FontFaceBuilder`] backing the [`fontsource_font_face!`] macro.
//!
//! [`fontsource_font_face!`]: crate::fontsource_font_face

use topcoat_core::runtime::cursor::ConstWriter;
use topcoat_font::runtime::{FontFace, FontSources, FontWeightRange};

use crate::{Family, Style, Subset};

/// jsDelivr origin every Fontsource file is served from.
const CDN_PREFIX: &str = "https://cdn.jsdelivr.net/fontsource/fonts/";
/// Pinned-to-latest segment that follows the family id.
const LATEST: &str = "@latest/";
/// File extension every face uses.
const EXT: &str = ".woff2";

/// A `const` builder that resolves a single Fontsource face — one
/// (subset, weight, style) of a [`Family`] — into a
/// [`FontFace`](topcoat_font::runtime::FontFace) with its CDN URL.
///
/// This is the machinery behind the [`fontsource_font_face!`] macro; reach for
/// the macro rather than calling the builder directly, since materializing the
/// URL as a `&'static str` requires the `const` items the macro expands to.
///
/// Every setter validates against the catalog in `const` context, so an
/// unsupported weight, style, or subset for the chosen family is a
/// compile-time error.
///
/// [`fontsource_font_face!`]: crate::fontsource_font_face
#[derive(Debug, Clone, Copy)]
pub struct FontFaceBuilder {
    family: Option<Family>,
    weight: Option<u16>,
    style: Option<Style>,
    subset: Option<Subset>,
}

impl FontFaceBuilder {
    /// Starts an empty builder. Set the family, weight, style, and subset
    /// before finishing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            family: None,
            weight: None,
            style: None,
            subset: None,
        }
    }

    /// Selects the [`Family`] to serve, e.g. [`ROBOTO`](crate::ROBOTO).
    #[must_use]
    pub const fn family(mut self, family: Family) -> Self {
        self.family = Some(family);
        self
    }

    /// Selects the weight, a number in `100..=900`.
    #[must_use]
    pub const fn weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Selects the [`Style`] (upright or italic).
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Selects the character [`Subset`] to serve.
    #[must_use]
    pub const fn subset(mut self, subset: Subset) -> Self {
        self.subset = Some(subset);
        self
    }

    /// Resolves and validates every field, panicking (a compile-time error in
    /// `const` use) when one is unset or unsupported by the family.
    const fn resolved(&self) -> (Family, u16, Style, Subset) {
        let Some(family) = self.family else {
            panic!("`fontsource_font_face!` is missing a `family`")
        };
        let Some(weight) = self.weight else {
            panic!("`fontsource_font_face!` is missing a `weight`")
        };
        let Some(style) = self.style else {
            panic!("`fontsource_font_face!` is missing a `style`")
        };
        let Some(subset) = self.subset else {
            panic!("`fontsource_font_face!` is missing a `subset`")
        };
        assert!(
            family.has_weight(weight),
            "the font family does not ship this weight"
        );
        assert!(
            family.has_style(style),
            "the font family does not ship this style"
        );
        assert!(
            family.has_subset(subset),
            "the font family does not ship this subset"
        );
        (family, weight, style, subset)
    }

    /// The exact byte length of the CDN URL, used to size the buffer
    /// [`url_bytes`](Self::url_bytes) fills.
    #[must_use]
    pub const fn url_len(&self) -> usize {
        let (family, weight, style, subset) = self.resolved();
        CDN_PREFIX.len()
            + family.id.len()
            + LATEST.len()
            + subset.as_str().len()
            + 1
            + decimal_len(weight)
            + 1
            + style.as_str().len()
            + EXT.len()
    }

    /// Writes the CDN URL into an exactly `N`-byte buffer, where `N` is
    /// [`url_len`](Self::url_len).
    ///
    /// `https://cdn.jsdelivr.net/fontsource/fonts/{id}@latest/{subset}-{weight}-{style}.woff2`
    #[must_use]
    pub const fn url_bytes<const N: usize>(&self) -> [u8; N] {
        let (family, weight, style, subset) = self.resolved();
        let mut buf = [0u8; N];
        let mut w = ConstWriter::new(&mut buf);
        w.write_bytes(CDN_PREFIX.as_bytes());
        w.write_bytes(family.id.as_bytes());
        w.write_bytes(LATEST.as_bytes());
        w.write_bytes(subset.as_str().as_bytes());
        w.write_bytes(b"-");
        write_decimal(&mut w, weight);
        w.write_bytes(b"-");
        w.write_bytes(style.as_str().as_bytes());
        w.write_bytes(EXT.as_bytes());
        buf
    }

    /// Assembles the final [`FontFace`] from `src` and the resolved
    /// weight/style descriptors.
    ///
    /// `src` is built at the macro's `const`-item site so the URL it carries is
    /// `&'static`; this only attaches the weight and style.
    #[must_use]
    pub const fn into_face(&self, src: FontSources) -> FontFace {
        let (family, weight, style, _subset) = self.resolved();
        FontFace::const_new(family.name, src)
            .with_weight(FontWeightRange::from_u16(weight, weight))
            .with_style(style.as_font_style())
    }
}

impl Default for FontFaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The number of decimal digits in `n` (e.g. `400` -> `3`).
const fn decimal_len(mut n: u16) -> usize {
    let mut len = 1;
    while n >= 10 {
        n /= 10;
        len += 1;
    }
    len
}

/// Writes `n` as decimal ASCII into `w`.
#[allow(clippy::cast_possible_truncation)]
const fn write_decimal(w: &mut ConstWriter<'_>, n: u16) {
    let len = decimal_len(n);
    // A `u16` is at most five decimal digits.
    let mut digits = [0u8; 5];
    let mut i = len;
    let mut rest = n;
    while i > 0 {
        i -= 1;
        digits[i] = b'0' + (rest % 10) as u8;
        rest /= 10;
    }
    w.write_bytes(digits.split_at(len).0);
}

#[cfg(test)]
mod tests {
    use crate::{ROBOTO, Style, Subset};

    use super::FontFaceBuilder;

    /// Renders a builder's CDN URL through the same `const` path the macro uses,
    /// using an ample fixed buffer in place of the macro's exact-length one.
    fn url(builder: FontFaceBuilder) -> String {
        let len = builder.url_len();
        let buf: [u8; 256] = builder.url_bytes::<256>();
        String::from_utf8(buf[..len].to_vec()).unwrap()
    }

    #[test]
    fn builds_static_cdn_url() {
        let builder = FontFaceBuilder::new()
            .family(ROBOTO)
            .weight(400)
            .style(Style::Normal)
            .subset(Subset::Latin);
        assert_eq!(
            url(builder),
            "https://cdn.jsdelivr.net/fontsource/fonts/roboto@latest/latin-400-normal.woff2",
        );
    }

    #[test]
    fn renders_weight_and_style_in_the_file_name() {
        let builder = FontFaceBuilder::new()
            .family(ROBOTO)
            .weight(700)
            .style(Style::Italic)
            .subset(Subset::Latin);
        assert_eq!(
            url(builder),
            "https://cdn.jsdelivr.net/fontsource/fonts/roboto@latest/latin-700-italic.woff2",
        );
    }

    #[test]
    #[should_panic = "does not ship this weight"]
    fn rejects_unavailable_weight() {
        let builder = FontFaceBuilder::new()
            .family(ROBOTO)
            .weight(250)
            .style(Style::Normal)
            .subset(Subset::Latin);
        let _ = builder.url_len();
    }

    #[test]
    fn macro_builds_a_const_face() {
        const _FACE: crate::FontFace =
            crate::fontsource_font_face!(ROBOTO, weight: 500, style: Style::Normal, subset: Subset::Latin);
    }

    #[test]
    #[cfg(feature = "asset")]
    fn macro_builds_a_self_hosted_const_face() {
        const _FACE: crate::FontFace = crate::fontsource_font_face!(
            ROBOTO,
            weight: 400,
            style: Style::Normal,
            subset: Subset::Latin,
            host: asset,
        );
    }
}
