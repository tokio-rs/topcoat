use std::{borrow::Cow, pin::Pin};

use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{
    identity::SiteKey,
    pass::{Children, Driver, RenderBuffer, View, mount, pass_boundary},
};

use crate::{
    Body, IntoPath, Methods, OwnedMethods, Path, Route, RouteFuture, content::Html,
    response::IntoResponse,
};

/// The async render function backing a [`PageFn`]: the page's component
/// future, rendering once per pass under the request's driver.
pub type PageRenderFn =
    for<'cx> fn(cx: &'cx Cx, body: Body) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'cx>>;

/// A page handler, backed by a plain render function, that renders a [`View`]
/// for a specific URL path.
///
/// Created either manually via `#[page("/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`RouterBuilder`](crate::RouterBuilder) alongside [`LayoutFn`]s, which wrap
/// it when their path is a prefix of the page's.
///
/// A page serves `GET` unless it declares other methods, either in the macro
/// (`#[page(POST "/path")]`) or through [`PageFn::new`].
#[derive(Debug, Clone)]
pub struct PageFn {
    /// The HTTP methods this page responds to.
    methods: OwnedMethods,
    /// The URL path this page handles.
    path: Cow<'static, Path>,
    /// The async render function that produces the page [`View`].
    render: PageRenderFn,
}

impl PageFn {
    /// Creates a new page with explicit methods, path, and render function.
    ///
    /// The methods are anything convertible into [`OwnedMethods`]: a single
    /// [`Method`](crate::Method), a `&'static [Method]`, a `Vec<Method>`, or
    /// [`Methods::Any`] to respond to every method.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    #[track_caller]
    pub fn new(
        methods: impl Into<OwnedMethods>,
        path: impl IntoPath,
        render: PageRenderFn,
    ) -> Self {
        Self::const_new(methods.into(), path.into_path(), render)
    }

    /// Const-context constructor used by macro-generated code.
    pub const fn const_new(
        methods: OwnedMethods,
        path: Cow<'static, Path>,
        render: PageRenderFn,
    ) -> Self {
        Self {
            methods,
            path,
            render,
        }
    }

    /// Returns the HTTP methods this page responds to.
    #[must_use]
    pub fn methods(&self) -> Methods<'_> {
        self.methods.as_methods()
    }

    /// Returns the URL path this page handles.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The page's component future.
    #[must_use]
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'cx>> {
        (self.render)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(PageFn);

/// The async render function backing a [`LayoutFn`]: the layout's component
/// future, receiving the token of the content it wraps and placing it in its
/// own render.
pub type LayoutRenderFn =
    for<'cx> fn(cx: &'cx Cx, slot: View) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'cx>>;

/// A layout handler, backed by a plain render function, that wraps pages whose
/// path starts with the layout's path prefix.
///
/// When multiple layouts match a page, they nest from most-specific (innermost)
/// to least-specific (outermost). For example, layouts at `/` and `/settings`
/// both match `/settings/profile`, rendering as: root -> settings -> page.
#[derive(Debug, Clone)]
pub struct LayoutFn {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The async render function that wraps the child content.
    render: LayoutRenderFn,
}

impl LayoutFn {
    /// Creates a new layout with an explicit path and render function.
    pub const fn new(path: Cow<'static, Path>, render: LayoutRenderFn) -> Self {
        Self { path, render }
    }

    /// Returns the path prefix this layout applies to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The layout's component future, wrapping the given child content.
    #[must_use]
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        slot: View,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'cx>> {
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
    fn methods(&self) -> Methods<'_> {
        self.page.methods()
    }

    fn path(&self) -> &Path {
        &self.page.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let mut driver = Driver::new(
                cx.detach(),
                compose(cx, self.page.render, &self.layouts, body),
            );
            let report = driver.render_to_end().await?;
            let mut response = Html(report.html).into_response(cx)?;
            if let Some(status_code) = report.status_code {
                *response.status_mut() = status_code;
            }
            response.headers_mut().extend(report.headers);
            Ok(response)
        })
    }
}

/// The request's component tree: the page and every layout are content
/// components of this composer, each layer placing the token of the layer
/// inside it, the outermost placed here. Layouts advance whether or not they
/// place their slot, so the page keeps rendering behind a layout that hides
/// it.
async fn compose(cx: &Cx, page: PageRenderFn, layouts: &[LayoutFn], body: Body) -> Result<()> {
    let mount = mount();
    let mut children = Children::new();
    let mut body = Some(body);
    loop {
        let mut out = RenderBuffer::new();
        let taken = &mut body;
        let mut token = children.content_keyed(
            SiteKey::new(file!(), line!(), column!(), 0),
            u32::MAX,
            || Ok(page(cx, taken.take().expect("the page is born once"))),
        )?;
        // Innermost (most specific) layout wraps first; the loop walks from
        // the page outward.
        for (depth, layout) in layouts.iter().enumerate().rev() {
            let inner = token;
            token = children.content_keyed(
                SiteKey::new(file!(), line!(), column!(), 1),
                depth as u32,
                || Ok(layout.render(cx, inner)),
            )?;
        }
        out.place(token)?;
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}
