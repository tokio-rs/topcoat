use std::pin::Pin;

use axum::response::IntoResponse;
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_view::runtime::View;

use crate::runtime::{Body, Layout, Path, Route};

pub trait Page: std::fmt::Debug + Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn render<'a>(
        &'a self,
        cx: &'a Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'a>>;
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

#[derive(Debug, Clone)]
pub struct PageWithLayouts {
    page: &'static dyn Page,
    layouts: Vec<&'static dyn Layout>,
}

impl PageWithLayouts {
    #[inline]
    pub fn new(page: &'static dyn Page, layouts: Vec<&'static dyn Layout>) -> Self {
        Self { page, layouts }
    }
}

impl Route for PageWithLayouts {
    #[inline]
    fn method(&self) -> http::Method {
        http::Method::GET
    }

    #[inline]
    fn path(&self) -> &Path {
        self.page.path()
    }

    fn handle<'a>(
        &'a self,
        cx: &'a Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<crate::runtime::Response>> + Send + 'a>> {
        Box::pin(async {
            let mut render = self.page.render(cx, body);
            for layout in self.layouts.iter().rev() {
                render = layout.render(cx, render);
            }
            let view = render.await?;
            Ok(axum::response::Html(view.render(cx)).into_response())
        })
    }
}
