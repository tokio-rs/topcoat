use topcoat_core::error::Result;
use topcoat_view::View;
use topcoat_view_macro::{component, view};

use crate::{Font, FontFormat, FontSource};

/// Loads a [`Font`] into the page. Place it in the page's `<head>`.
///
/// Renders a stylesheet `<link>` for the font's `@font-face` rules. By default
/// it also renders a `rel="preload"` `<link>` for the first source of each
/// face, so the browser starts downloading the font files before it parses the
/// stylesheet. The browser downloads every preloaded file, even for faces the
/// page does not use, so pass `preload: false` for fonts with many faces.
///
/// The font must be registered on the router, either through discovery or
/// with [`RouterBuilderFontExt::font`](crate::RouterBuilderFontExt::font).
///
/// ```rust
/// # use topcoat::{font::{Font, fontsource::fontsource_font}, view::{View, view}};
/// # const LAVISHLY_YOURS: Font = fontsource_font!(LAVISHLY_YOURS, host: Asset);
/// # #[topcoat::view::component]
/// # async fn example() -> topcoat::Result<impl View> {
/// Ok(view! {
///     topcoat::font::link(font: LAVISHLY_YOURS)
/// })
/// # }
/// ```
#[component]
pub async fn link(
    /// The font to load.
    font: Font,
    /// Whether to render `rel="preload"` links for the font files.
    #[default(true)]
    preload: bool,
) -> Result<impl View> {
    Ok(view! {
        if preload {
            for face in font.faces().iter() {
                if let Some(source) = face.src().first() {
                    preload_link(source: source)
                }
            }
        }
        <link rel="stylesheet" href=(font)>
    })
}

/// Renders a `rel="preload"` `<link>` for a single [`FontSource`].
///
/// Renders nothing for a [`Local`](FontSource::Local) source. The `type`
/// attribute is set from the source's format, and left out when the source has
/// no format.
#[component]
pub async fn preload_link(
    /// The font source to preload.
    source: &FontSource,
) -> Result<impl View> {
    Ok(view! {
        if let FontSource::Url { url, format, .. } = source {
            <link
                rel="preload"
                href=(url.clone())
                as="font"
                type=(format.map(FontFormat::mime_type))
                crossorigin="true"
            >
        }
    })
}
