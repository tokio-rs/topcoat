use topcoat_core::runtime::context::Cx;
use topcoat_router::runtime::{
    Body, IntoResponse, Method, Path, PathBuf, Route, RouteFuture, RouterBuilder,
};

use crate::Font;

const FONT_ROUTE_PREFIX: &str = "/_topcoat/fonts";

pub struct FontRoute {
    path: PathBuf,
    font: Font,
}

impl FontRoute {
    #[must_use]
    pub fn new(font: Font) -> Self {
        Self {
            path: Path::new(&format!(
                "{FONT_ROUTE_PREFIX}/{}-{:016x}.css",
                font.family(),
                font.hash()
            ))
            .to_owned(),
            font,
        }
    }
}

impl Route for FontRoute {
    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        Box::pin(async {
            let mut response = String::new();
            let _ = self.font.faces().fmt(cx, &mut response);
            response.into_response(cx)
        })
    }
}

pub trait RouterBuilderFontExt {
    #[must_use]
    fn font(self, font: Font) -> Self;

    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_fonts(self) -> Self;
}

impl RouterBuilderFontExt for RouterBuilder {
    fn font(mut self, font: Font) -> Self {
        self = self.route(FontRoute::new(font));
        self
    }

    #[cfg(feature = "discover")]
    fn discover_fonts(mut self) -> Self {
        for font in inventory::iter::<crate::FontFn> {
            self = self.font(font.build());
        }
        self
    }
}
