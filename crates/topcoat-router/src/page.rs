use std::{borrow::Cow, sync::Arc};

use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{BoxView, NodeViewPartsStream, NodeWriter, ViewStream};

use crate::{
    Body, IntoPath, Methods, OwnedMethods, Path, Route, RouteFuture, RouteId,
    content::ViewResponse, response::IntoResponse, route,
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

    /// Renders the page to a [`View`](topcoat_view::View).
    ///
    /// The returned view owns its request context: it may borrow `self`, but
    /// not `cx`. An error in the page body is yielded through the view's
    /// stream rather than returned here.
    fn render<'s>(&'s self, cx: &Cx, body: Body) -> BoxView<'s>;

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

    fn render<'s>(&'s self, cx: &Cx, body: Body) -> BoxView<'s> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// The render function backing a [`PageFn`].
pub type PageRenderFn = fn(cx: &Cx, body: Body) -> BoxView<'static>;

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

    fn render<'s>(&'s self, cx: &Cx, body: Body) -> BoxView<'s> {
        (self.render)(cx, body)
    }
}

/// The content a [`Layout`] wraps: the page, already composed with any inner
/// layouts, waiting to be rendered against a request context.
///
/// Interpolating the slot into a `view!` template renders it against the
/// template's context, so a layout can provide request context values to
/// everything beneath it by deriving a context with
/// [`Cx::with`](topcoat_core::context::Cx::with) and rebinding the template's
/// context (`view! { cx => ... }`).
pub struct Slot<'a> {
    render: Box<dyn FnOnce(&Cx) -> BoxView<'a> + Send + 'a>,
}

impl<'a> Slot<'a> {
    /// Wraps the function that renders the slot's content.
    #[must_use]
    pub fn new(render: impl FnOnce(&Cx) -> BoxView<'a> + Send + 'a) -> Self {
        Self {
            render: Box::new(render),
        }
    }

    /// Renders the slot's content against `cx`.
    ///
    /// The returned view owns `cx`'s request context, so it outlives the
    /// borrow.
    #[must_use]
    pub fn render(self, cx: &Cx) -> BoxView<'a> {
        (self.render)(cx)
    }
}

/// Interpolating the slot into a template renders it against the template's
/// context.
impl NodeViewPartsStream for Slot<'_> {
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(self, cx: &'cx Cx, writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        self.render(cx).into_view_parts_stream(cx, writer).await
    }
}

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

    /// Renders the layout, embedding the given child content [`Slot`].
    ///
    /// The returned view owns its request context: it may borrow `self` and
    /// the slot, but not `cx`.
    fn render<'s>(&'s self, cx: &Cx, slot: Slot<'s>) -> BoxView<'s>;
}

impl<L: Layout + ?Sized> Layout for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn render<'s>(&'s self, cx: &Cx, slot: Slot<'s>) -> BoxView<'s> {
        (**self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// The render function backing a [`LayoutFn`], receiving the child content as
/// a [`Slot`].
pub type LayoutRenderFn = for<'a> fn(cx: &Cx, slot: Slot<'a>) -> BoxView<'a>;

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

    fn render<'s>(&'s self, cx: &Cx, slot: Slot<'s>) -> BoxView<'s> {
        (self.render)(cx, slot)
    }
}

/// A [`Page`] paired with the [`Layout`]s that wrap it.
pub struct PageWithLayouts {
    inner: Arc<PageWithLayoutsInner>,
}

/// The pair behind one shared handle, so the response body stream can own it
/// past the handler.
struct PageWithLayoutsInner {
    page: Box<dyn Page>,
    /// The matching layouts, ordered by ascending path length (outermost first).
    layouts: Vec<Arc<dyn Layout>>,
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
        Box::pin(async move {
            let stream = {
                let inner = self.inner.clone();
                let cx = cx.clone();
                ViewStream::new(async move {
                    // The page is the innermost slot, each layout wraps the
                    // slot beneath it, and only the outermost layer renders
                    // directly against the request context. Nothing runs
                    // until the view is driven: each layout decides when its
                    // slot renders and which context it sees.
                    let page = &inner.page;
                    let mut slot = Slot::new(move |cx| page.render(cx, body));
                    let mut layouts = inner.layouts.iter();
                    let outermost = layouts.next();
                    for layout in layouts.rev() {
                        slot = Slot::new(move |cx| layout.render(cx, slot));
                    }
                    let view = match outermost {
                        Some(layout) => layout.render(&cx, slot),
                        None => slot.render(&cx),
                    };
                    topcoat_view::internal::forward(view).await;
                    Ok(())
                })
            };

            ViewResponse::try_from(Box::pin(stream))
                .await?
                .into_response(cx)
        })
    }
}
