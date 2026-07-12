use topcoat_core::error::Result;
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::{Font, FontFormat, FontSource};

/// Loads a [`Font`] into the page.
///
/// Renders the stylesheet `<link>` that pulls in the font's `@font-face` rules,
/// and, by default, a `rel="preload"` `<link>` for the first source of each
/// face so the browser can start fetching the files before the CSS is parsed.
///
/// ```rust
/// # use topcoat::{font::{Font, fontsource::fontsource_font}, view::view};
/// # const LAVISHLY_YOURS: Font = fontsource_font!(LAVISHLY_YOURS, host: Asset);
/// # #[topcoat::view::component]
/// # async fn example() -> topcoat::Result {
/// view! {
///     topcoat::font::link(font: LAVISHLY_YOURS)
/// }
/// # }
/// ```
#[component]
pub async fn link(
    /// The font to load.
    font: Font,
    /// Whether to emit `rel="preload"` links for the font's sources ahead of
    /// the stylesheet.
    #[default(true)]
    preload: bool,
) -> Result<View> {
    view! {
        if preload {
            for face in font.faces().iter() {
                if let Some(source) = face.src().first() {
                    preload_link(source: source)
                }
            }
        }
        <link rel="stylesheet" href=(font)>
    }
}

/// Renders a `rel="preload"` `<link>` for a single font [`FontSource`].
///
/// Emits nothing for sources that are not URL-backed (such as local fonts). The
/// `type` attribute is set from the source's format when known and omitted
/// otherwise.
#[component]
pub async fn preload_link(
    /// The font source to preload.
    source: &FontSource,
) -> Result<View> {
    view! {
        if let FontSource::Url { url, format, .. } = source {
            <link
                rel="preload"
                href=(url.clone())
                as="font"
                type=(format.map(FontFormat::mime_type))
                crossorigin="true"
            >
        }
    }
}
