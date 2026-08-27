use std::{borrow::Cow, sync::Arc};

use topcoat_core::context::Cx;
use topcoat_view::{
    BoxView, Child, ViewBuffer,
    internal::{MoveView, drive_sealed},
};

use crate::{
    Body, IntoPath, Methods, OwnedMethods, Path, Route, RouteFuture, RouteId,
    response::AsyncIntoResponse, route,
};

/// A page handler that renders a [`View`](topcoat_view::View) for a specific
/// URL path.
///
/// Registered into a [`RouterBuilder`](crate::RouterBuilder) with
/// [`page`](crate::RouterBuilder::page), alongside [`Layout`]s, which wrap it
/// when their path is a prefix of the page's.
///
/// A page serves `GET` unless it declares other methods.
pub trait Page: Send + Sync + 'static {
    /// The identity of this page's handler.
    fn id(&self) -> RouteId;

    /// The HTTP methods this page responds to.
    fn methods(&self) -> Methods<'_>;

    /// The URL path this page handles.
    fn path(&self) -> &Path;

    /// Renders the page to a [`View`](topcoat_view::View) building into
    /// `buf` under the request context `cx`.
    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, body: Body) -> BoxView<'a>;

    /// Returns whether this page handles the current request.
    ///
    /// Only the handler is compared, so a page is current for every value its
    /// path parameters take, whatever the request's query or fragment.
    ///
    /// # Panics
    ///
    /// Panics if the request matched no route: either its path matched no
    /// endpoint, or the endpoint holds no route for the request's method.
    fn is_current(&self, cx: &Cx) -> bool {
        route(cx).id() == self.id()
    }
}

impl<P: Page + ?Sized> Page for &'static P {
    fn id(&self) -> RouteId {
        (**self).id()
    }

    fn methods(&self) -> Methods<'_> {
        (**self).methods()
    }

    fn path(&self) -> &Path {
        (**self).path()
    }

    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, body: Body) -> BoxView<'a> {
        (**self).render(cx, buf, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// The render function backing a [`PageFn`].
pub type PageRenderFn = for<'a> fn(cx: &'a Cx, buf: &'a ViewBuffer, body: Body) -> BoxView<'a>;

/// A [`Page`] backed by a plain render function.
///
/// Turns a function into a page without implementing [`Page`] on a struct,
/// pairing it with the methods and path it serves.
#[derive(Debug, Clone)]
pub struct PageFn {
    /// The identity of this page's handler.
    id: RouteId,
    /// The HTTP methods this page responds to.
    methods: OwnedMethods,
    /// The URL path this page handles.
    path: Cow<'static, Path>,
    /// The render function that produces the page [`View`](topcoat_view::View).
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
        Self {
            id: RouteId::new(),
            methods: methods.into(),
            path: path.into_path(),
            render,
        }
    }
}

impl Page for PageFn {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        self.methods.as_methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, body: Body) -> BoxView<'a> {
        (self.render)(cx, buf, body)
    }
}

/// The content a [`Layout`] wraps: the page, already composed with any inner
/// layouts.
pub type Slot<'a> = Child<'a>;

/// A layout handler that wraps pages whose path starts with the layout's path
/// prefix.
///
/// When multiple layouts match a page, they nest from most-specific (innermost)
/// to least-specific (outermost). For example, layouts at `/` and `/settings`
/// both match `/settings/profile`, rendering as: root -> settings -> page.
///
/// Registered into a [`RouterBuilder`](crate::RouterBuilder) with
/// [`layout`](crate::RouterBuilder::layout).
pub trait Layout: Send + Sync + 'static {
    /// The path prefix this layout applies to.
    fn path(&self) -> &Path;

    /// Renders the layout, embedding the given child content [`Slot`], to a
    /// [`View`](topcoat_view::View) building into `buf` under the request
    /// context `cx`.
    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, slot: Slot<'a>) -> BoxView<'a>;
}

impl<L: Layout + ?Sized> Layout for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, slot: Slot<'a>) -> BoxView<'a> {
        (**self).render(cx, buf, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// The render function backing a [`LayoutFn`], receiving the child content as
/// a [`Slot`].
pub type LayoutRenderFn =
    for<'a> fn(cx: &'a Cx, buf: &'a ViewBuffer, slot: Slot<'a>) -> BoxView<'a>;

/// A [`Layout`] backed by a plain render function.
///
/// Turns a function into a layout without implementing [`Layout`] on a
/// struct, pairing it with the path prefix it applies to.
#[derive(Debug, Clone)]
pub struct LayoutFn {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The render function that wraps the child content [`Slot`].
    render: LayoutRenderFn,
}

impl LayoutFn {
    /// Creates a new layout with an explicit path and render function.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    #[track_caller]
    pub fn new(path: impl IntoPath, render: LayoutRenderFn) -> Self {
        Self {
            path: path.into_path(),
            render,
        }
    }
}

impl Layout for LayoutFn {
    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, slot: Slot<'a>) -> BoxView<'a> {
        (self.render)(cx, buf, slot)
    }
}

/// A [`Page`] paired with the [`Layout`]s that wrap it.
pub struct PageWithLayouts {
    inner: Arc<PageWithLayoutsInner>,
}

/// The pair behind one shared handle, so the response can own it past the
/// handler.
struct PageWithLayoutsInner {
    page: Box<dyn Page>,
    /// The matching layouts, ordered by ascending path length (outermost first).
    layouts: Vec<Arc<dyn Layout>>,
}

impl PageWithLayoutsInner {
    /// Composes the page with its layouts: the page is the innermost slot,
    /// each layout wraps the slot beneath it, and the outermost layout is
    /// the view.
    fn render<'a>(&'a self, cx: &'a Cx, buf: &'a ViewBuffer, body: Body) -> BoxView<'a> {
        let mut view = self.page.render(cx, buf, body);
        for layout in self.layouts.iter().rev() {
            view = layout.render(cx, buf, Slot::new(view));
        }
        view
    }
}

impl PageWithLayouts {
    /// Pairs `page` with the `layouts` that wrap it.
    ///
    /// `layouts` must be ordered from least- to most-specific (ascending path
    /// length); they are applied from the innermost (most specific) outward.
    #[must_use]
    pub fn new(page: Box<dyn Page>, layouts: Vec<Arc<dyn Layout>>) -> Self {
        Self {
            inner: Arc::new(PageWithLayoutsInner { page, layouts }),
        }
    }
}

impl Route for PageWithLayouts {
    fn id(&self) -> RouteId {
        self.inner.page.id()
    }

    fn methods(&self) -> Methods<'_> {
        self.inner.page.methods()
    }

    fn path(&self) -> &Path {
        self.inner.page.path()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        // The response body outlives the handler, so the view owns the pair,
        // a copy of the request context, and the buffer it builds in, and
        // drives itself in place.
        let inner = Arc::clone(&self.inner);
        let owned = cx.clone();
        Box::pin(async move {
            let view = MoveView::new(async move {
                let buf = ViewBuffer::new();
                let view = inner.render(&owned, &buf, body);
                drive_sealed(&buf, view).await
            });
            view.async_into_response(cx).await
        })
    }
}
