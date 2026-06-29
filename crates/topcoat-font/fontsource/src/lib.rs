mod builder;
mod family;
mod style;
mod subset;

pub use builder::*;
pub use family::*;
pub use style::*;
pub use subset::*;

pub use topcoat_font::runtime::FontFace;

/// Implementation details the [`fontsource_font_face!`] macro expands into. Not
/// part of the public API.
#[doc(hidden)]
pub mod __private {
    pub use topcoat_font::runtime::{FontFormat, FontSource, FontSources};

    #[cfg(feature = "asset")]
    pub use topcoat_asset::{Asset, asset};
}

/// Builds a [`FontFace`] for one Fontsource face, served from the jsDelivr CDN.
///
/// Choose the family, weight, style, and subset; the macro generates the CDN
/// URL and assembles the face. The weight, style, and subset are validated
/// against the catalog at compile time, so an unsupported combination fails to
/// build. The result is a `const`, usable in a `const` or `static`.
///
/// By default the face is served straight from the jsDelivr CDN. Pass
/// `host: asset` to instead declare the file as a Topcoat
/// [`asset!`](topcoat_asset::asset), so the bundler downloads it and serves a
/// content-hashed copy from your own origin. The `asset` arm requires the
/// `asset` feature.
///
/// ```
/// use topcoat_font_fontsource::{FontFace, ROBOTO, Style, Subset, fontsource_font_face};
///
/// const REGULAR: FontFace =
///     fontsource_font_face!(ROBOTO, weight: 400, style: Style::Normal, subset: Subset::Latin);
/// ```
#[macro_export]
macro_rules! fontsource_font_face {
    ($family:expr, weight: $weight:expr, style: $style:expr, subset: $subset:expr $(,)?) => {{
        const URL: &str = $crate::__fontsource_url!($family, $weight, $style, $subset);
        const SRC: $crate::__private::FontSources = $crate::__private::FontSources::const_new(
            const {
                &[$crate::__private::FontSource::url_str(
                    URL,
                    ::core::option::Option::Some($crate::__private::FontFormat::Woff2),
                    ::core::option::Option::None,
                )]
            },
        );
        $crate::FontFaceBuilder::new()
            .family($family)
            .weight($weight)
            .style($style)
            .subset($subset)
            .into_face(SRC)
    }};
    ($family:expr, weight: $weight:expr, style: $style:expr, subset: $subset:expr, host: asset $(,)?) => {{
        const URL: &str = $crate::__fontsource_url!($family, $weight, $style, $subset);
        const ASSET: $crate::__private::Asset = $crate::__private::asset!(URL);
        const SRC: $crate::__private::FontSources = $crate::__private::FontSources::const_new(
            const {
                &[$crate::__private::FontSource::url_asset(
                    ASSET,
                    ::core::option::Option::Some($crate::__private::FontFormat::Woff2),
                    ::core::option::Option::None,
                )]
            },
        );
        $crate::FontFaceBuilder::new()
            .family($family)
            .weight($weight)
            .style($style)
            .subset($subset)
            .into_face(SRC)
    }};
}

/// Materializes a Fontsource CDN URL as a `&'static str` `const`. Shared by both
/// arms of [`fontsource_font_face!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __fontsource_url {
    ($family:expr, $weight:expr, $style:expr, $subset:expr) => {{
        const BUILDER: $crate::FontFaceBuilder = $crate::FontFaceBuilder::new()
            .family($family)
            .weight($weight)
            .style($style)
            .subset($subset);
        const LEN: usize = BUILDER.url_len();
        const BYTES: [u8; LEN] = BUILDER.url_bytes::<LEN>();
        match ::core::str::from_utf8(&BYTES) {
            ::core::result::Result::Ok(url) => url,
            ::core::result::Result::Err(_) => ::core::panic!("font url was not valid utf-8"),
        }
    }};
}
