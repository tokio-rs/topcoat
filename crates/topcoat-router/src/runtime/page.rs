use std::borrow::Cow;
use std::pin::Pin;

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_view::runtime::View;

use crate::runtime::{Body, Html, IntoResponse, Path, Route, RouteFuture};

/// The async render function backing a [`PageFn`].
pub type PageRenderFn = for<'cx> fn(
    cx: &'cx Cx,
    body: Body,
) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

/// A page handler, backed by a plain render function, that renders a [`View`]
/// for a specific URL path.
///
/// Created either manually via `#[page("/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`RouterBuilder`](crate::runtime::RouterBuilder) alongside [`LayoutFn`]s, which wrap
/// it when their path is a prefix of the page's.
#[derive(Debug, Clone)]
pub struct PageFn {
    /// The URL path this page handles.
    path: Cow<'static, Path>,
    /// The async render function that produces the page [`View`].
    render: PageRenderFn,
}

impl PageFn {
    /// Creates a new page with an explicit path and render function.
    pub const fn new(path: Cow<'static, Path>, render: PageRenderFn) -> Self {
        Self { path, render }
    }

    /// Returns the URL path this page handles.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Renders the page, returning a [`Result`].
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>> {
        (self.render)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(PageFn);

/// The async render function backing a [`LayoutFn`], receiving a [`Slot`] for child content.
pub type LayoutRenderFn = for<'cx> fn(
    cx: &'cx Cx,
    slot: Slot<'cx>,
) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

/// A future that resolves to the inner page (or nested layout) [`Result`].
///
/// Every layout render function receives a `Slot` and `.await`s it to embed
/// the child content at the desired location.
pub type Slot<'cx> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

/// A layout handler, backed by a plain render function, that wraps pages whose
/// path starts with the layout's path prefix.
///
/// When multiple layouts match a page, they nest from most-specific (innermost)
/// to least-specific (outermost). For example, layouts at `/` and `/settings`
/// both match `/settings/profile`, rendering as: root → settings → page.
#[derive(Debug, Clone)]
pub struct LayoutFn {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The async render function that wraps child content via a [`Slot`].
    render: LayoutRenderFn,
}

impl LayoutFn {
    /// Creates a new layout with an explicit path and render function.
    pub const fn new(path: Cow<'static, Path>, render: LayoutRenderFn) -> Self {
        Self { path, render }
    }

    /// Returns the path prefix this layout applies to.
    #[must_use]
    pub fn path(&self) -> Cow<'static, Path> {
        self.path.clone()
    }

    /// Renders the layout, embedding the given [`Slot`] as child content.
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        slot: Slot<'cx>,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>> {
        (self.render)(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(LayoutFn);

/// A [`PageFn`] paired with the [`LayoutFn`]s that wrap it.
pub struct PageWithLayouts {
    page: PageFn,
    /// The matching layouts, ordered by ascending path length (outermost first).
    layouts: Vec<LayoutFn>,
}

impl PageWithLayouts {
    /// Pairs `page` with the `layouts` that wrap it.
    ///
    /// `layouts` must be ordered from least- to most-specific (ascending path
    /// length); they are applied from the innermost (most specific) outward.
    #[must_use]
    pub fn new(page: PageFn, layouts: Vec<LayoutFn>) -> Self {
        Self { page, layouts }
    }
}

impl Route for PageWithLayouts {
    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> Cow<'static, Path> {
        self.page.path.clone()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let mut render = self.page.render(cx, body);
            for layout in self.layouts.iter().rev() {
                render = layout.render(cx, render);
            }
            let view = render.await?;
            Html(view.render(cx)).into_response()
        })
    }
}
