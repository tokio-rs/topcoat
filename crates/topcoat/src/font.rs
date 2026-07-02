pub use topcoat_font::runtime::*;
pub use topcoat_font_macro::*;

#[cfg(feature = "font-fontsource")]
pub mod fontsource {
    pub use topcoat_font_fontsource::runtime::*;
    pub use topcoat_font_fontsource_macro::*;
}

#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn link(font: Font, #[default(true)] preload: bool) -> topcoat::Result {
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

#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn preload_link(source: &FontSource) -> topcoat::Result {
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
