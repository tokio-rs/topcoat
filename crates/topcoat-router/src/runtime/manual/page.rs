use std::sync::Arc;

use axum::response::{Html, IntoResponse};
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_view::runtime::View;

use crate::runtime::{Body, Layout, Path, Response, Route, RouteHandlerFuture};

/// The future returned by [`Page::render`].
pub type PageRenderFuture<'a> = std::pin::Pin<Box<dyn Future<Output = Result<View>> + Send + 'a>>;

pub trait Page: std::fmt::Debug + Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> PageRenderFuture<'a>;
}

impl<P> Page for &'static P
where
    P: Page + ?Sized,
{
    #[inline]
    fn path(&self) -> &Path {
        (*self).path()
    }

    #[inline]
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> PageRenderFuture<'a> {
        (*self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// A [`Page`] paired with the [`Layout`]s that wrap it, registered as a [`Route`]
/// that renders the page nested inside its layouts.
#[derive(Debug, Clone)]
pub struct PageWithLayouts {
    page: Arc<dyn Page>,
    layouts: Vec<Arc<dyn Layout>>,
}

impl PageWithLayouts {
    pub fn new(page: Arc<dyn Page>, layouts: Vec<Arc<dyn Layout>>) -> Self {
        Self { page, layouts }
    }
}

impl Route for PageWithLayouts {
    fn method(&self) -> http::Method {
        http::Method::GET
    }

    fn path(&self) -> &Path {
        self.page.path()
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a> {
        Box::pin(async move {
            let mut render = self.page.render(cx, body);
            for layout in self.layouts.iter().rev() {
                render = layout.render(cx, render);
            }
            let view = render.await?;
            Ok::<Response, _>(Html(view.render(cx)).into_response())
        })
    }
}
