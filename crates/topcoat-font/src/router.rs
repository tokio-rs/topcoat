use std::sync::OnceLock;

use topcoat_core::context::Cx;
use topcoat_router::{
    Body, HeaderValue, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId, RouterBuilder,
    header::{CACHE_CONTROL, CONTENT_TYPE},
    response::Response,
};

use crate::{Font, FontResolver};

const FONT_ROUTE_PREFIX: &str = "/_topcoat/fonts";

/// `Cache-Control` applied to every served font. Bundled fonts carry a
/// content hash, so their contents never change for a given URL.
const CACHE_CONTROL_VALUE: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");

/// The URL path a font's stylesheet is served at, e.g.
/// `/_topcoat/fonts/Lavishly-Yours-1a2b3c4d5e6f7a8b.css`.
///
/// The family name is slugified to stay URL-safe (so the served route and the
/// rendered `href` match without percent-encoding), and a content hash keeps the
/// URL immutable for a given set of faces.
fn font_route_path(font: Font, write: &mut dyn std::fmt::Write) -> std::fmt::Result {
    write.write_str(FONT_ROUTE_PREFIX)?;
    write.write_str("/")?;
    for ch in font.family().chars() {
        write.write_char(if ch.is_ascii_alphanumeric() { ch } else { '-' })?;
    }
    write!(write, "-{:016x}.css", font.hash())
}

/// A route that serves a [`Font`]'s `@font-face` rules as a CSS stylesheet.
///
/// The route answers `GET` requests at a path under `/_topcoat/fonts/` that
/// contains the family name and the font's [`hash`](Font::hash). Browsers may
/// cache the response for a year, because changing the font also changes the
/// path. [`RouterBuilderFontExt::font`] adds this route for you.
pub struct FontRoute {
    id: RouteId,
    path: PathBuf,
    font: Font,
    cache: OnceLock<String>,
}

impl FontRoute {
    /// Creates the route for `font`.
    #[must_use]
    pub fn new(font: Font) -> Self {
        let mut path = String::new();
        let _ = font_route_path(font, &mut path);
        Self {
            id: RouteId::new(),
            path: Path::new(&path).to_owned(),
            font,
            cache: OnceLock::new(),
        }
    }
}

impl Route for FontRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Only(&[Method::GET])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        Box::pin(async {
            // Render the `@font-face` CSS once and cache the result.
            let cached_css = self.cache.get_or_init(|| {
                let mut css = String::new();
                let _ = self.font.faces().fmt(cx, &mut css);
                css
            });

            let mut response = Response::new(Body::from(cached_css.clone()));
            let headers = response.headers_mut();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            );
            headers.insert(CACHE_CONTROL, CACHE_CONTROL_VALUE);
            Ok(response)
        })
    }
}

/// Registers [`Font`]s on a [`RouterBuilder`].
///
/// A font must be registered before a page can load it. Registering a font
/// adds a route that serves its stylesheet and sets up the app context that
/// views use to render the stylesheet URL.
pub trait RouterBuilderFontExt {
    /// Registers `font` so that its stylesheet is served by the router.
    #[must_use]
    fn font(self, font: Font) -> Self;

    /// Registers every font declared with the `font!` or `fontsource_font!`
    /// macro anywhere in the program.
    ///
    /// The router's `discover` method calls this for you.
    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_fonts(self) -> Self;
}

impl RouterBuilderFontExt for RouterBuilder {
    fn font(mut self, font: Font) -> Self {
        self = self.route(FontRoute::new(font));
        // Every font shares the same resolver, so register it only for the
        // first one; a second `app_context` of the same type would panic.
        if self.get_app_context::<FontResolver>().is_none() {
            self = self.app_context(FontResolver::new(Box::new(font_route_path)));
        }
        self
    }

    #[cfg(feature = "discover")]
    fn discover_fonts(mut self) -> Self {
        for font in inventory::iter::<crate::Font> {
            self = self.font(*font);
        }
        self
    }
}
