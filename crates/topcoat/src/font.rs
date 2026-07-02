pub use topcoat_font::runtime::*;
pub use topcoat_font_macro::*;

#[cfg(feature = "font-fontsource")]
pub mod fontsource {
    pub use topcoat_font_fontsource::runtime::*;
    pub use topcoat_font_fontsource_macro::*;
}

/// Loads a [`Font`] into the page.
///
/// Renders the stylesheet `<link>` that pulls in the font's `@font-face` rules,
/// and, by default, a `rel="preload"` `<link>` for the first source of each
/// face so the browser can start fetching the files before the CSS is parsed.
///
/// ```rust
/// # use topcoat::{font::{Font, fontsource::fontsource_font}, view::view};
/// # const LAVISHLY_YOURS: Font = fontsource_font!("Lavishly Yours", host: Asset);
/// # #[topcoat::view::component]
/// # async fn example() -> topcoat::Result {
/// view! {
///     topcoat::font::link(font: LAVISHLY_YOURS)
/// }
/// # }
/// ```
#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn link(
    /// The font to load.
    font: Font,
    /// Whether to emit `rel="preload"` links for the font's sources ahead of
    /// the stylesheet.
    #[default(true)]
    preload: bool,
) -> topcoat::Result {
    topcoat::view::view! {
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
#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn preload_link(
    /// The font source to preload.
    source: &FontSource,
) -> topcoat::Result {
    topcoat::view::view! {
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
