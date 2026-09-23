use std::{borrow::Cow, sync::Arc};

use topcoat_core::context::Cx;
use topcoat_view::{
    BoxView, Child,
    internal::{MoveView, ScopeView},
};

use crate::{
    Body, IntoPath, Methods, OwnedMethods, Path, Route, RouteFuture, RouteId,
    response::AsyncIntoResponse, route,
};

/// A handler that renders a [`View`](topcoat_view::View) for a URL path.
///
/// Every [`Layout`] whose path is a prefix of the page's path wraps the
/// rendered view. `#[page]` with an absolute path implements this trait.
/// Register a page with [`RouterBuilder::page`](crate::RouterBuilder::page),
/// or use [`PageFn`] to make one from a function.
pub trait Page: Send + Sync + 'static {
    /// Returns the id of this page's handler.
    ///
    /// Return the same [`RouteId`] on every call.
    fn id(&self) -> RouteId;

    /// Returns the HTTP methods this page responds to.
    fn methods(&self) -> Methods<'_>;

    /// Returns the path pattern this page handles.
    fn path(&self) -> &Path;

    /// Renders the page for the request `cx` belongs to.
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a>;

    /// Returns whether this page is the one handling the current request.
    ///
    /// Only the page's id is compared, so the result does not depend on the
    /// values of path parameters or on the query string.
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

    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// The render function of a [`PageFn`].
pub type PageRenderFn = for<'a> fn(cx: &'a Cx, body: Body) -> BoxView<'a>;

/// A [`Page`] made from a render function, a path, and a set of methods.
///
/// Use it to register a page without implementing [`Page`] on a type of your
/// own.
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
    /// Creates a page that runs `render` for requests to `path` with one of
    /// `methods`.
    ///
    /// `methods` accepts anything that converts into [`OwnedMethods`], like a
    /// single [`Method`](crate::Method), a slice or `Vec` of methods, or
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

    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a> {
        (self.render)(cx, body)
    }
}

/// The content a [`Layout`] wraps: the page, already wrapped in any inner
/// layouts.
pub type Slot<'a> = Child<'a>;

/// A handler that wraps every page whose path starts with the layout's path.
///
/// When several layouts match a page, the layout with the longer path is
/// nested inside the one with the shorter path. For example, layouts at `/`
/// and `/settings` both match `/settings/profile`, which renders as
/// root -> settings -> page.
///
/// `#[layout]` with an absolute path implements this trait. Register a
/// layout with [`RouterBuilder::layout`](crate::RouterBuilder::layout), or
/// use [`LayoutFn`] to make one from a function.
pub trait Layout: Send + Sync + 'static {
    /// Returns the path whose pages this layout wraps.
    fn path(&self) -> &Path;

    /// Renders the layout around `slot`, the content it wraps, for the
    /// request `cx` belongs to.
    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a>;
}

impl<L: Layout + ?Sized> Layout for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a> {
        (**self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// The render function of a [`LayoutFn`]. It receives the wrapped content as
/// a [`Slot`].
pub type LayoutRenderFn = for<'a> fn(cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a>;

/// A [`Layout`] made from a render function and a path.
///
/// Use it to register a layout without implementing [`Layout`] on a type of
/// your own.
#[derive(Debug, Clone)]
pub struct LayoutFn {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The render function that wraps the child content [`Slot`].
    render: LayoutRenderFn,
}

impl LayoutFn {
    /// Creates a layout that runs `render` for the pages under `path`.
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

    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a> {
        (self.render)(cx, slot)
    }
}

/// A [`Page`] together with the [`Layout`]s that wrap it, served as a
/// [`Route`].
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
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a> {
        let mut view = self.page.render(cx, body);
        for layout in self.layouts.iter().rev() {
            view = layout.render(cx, Slot::new(view));
        }
        view
    }
}

impl PageWithLayouts {
    /// Combines `page` with the `layouts` that wrap it.
    ///
    /// Order `layouts` from outermost to innermost, which is by ascending path
    /// length.
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
        // The response body outlives the handler, so the view owns the pair
        // and a copy of the request context, and drives itself in place as
        // the outermost view of the build.
        let inner = Arc::clone(&self.inner);
        let owned = cx.clone();
        Box::pin(async move {
            let view = MoveView::new(async move {
                let view = inner.render(&owned, body);
                MoveView::drive(ScopeView::new(view)).await
            });
            view.async_into_response(cx).await
        })
    }
}
