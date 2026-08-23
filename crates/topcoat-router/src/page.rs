use std::{borrow::Cow, pin::Pin, sync::Arc};

use futures_core::Stream;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{ViewChunk, ViewStream};

use crate::{
    Body, IntoPath, Methods, OwnedMethods, Path, Route, RouteFuture, RouteId,
    content::ViewResponse, response::IntoResponse, route,
};

/// The stream returned by [`Page::render`] and [`Layout::render`]: a boxed,
/// `Send` stream borrowing the handler and its request context.
pub type PageViewStream<'cx> = Pin<Box<dyn Stream<Item = Result<ViewChunk>> + Send + 'cx>>;

/// A page handler that renders a [`View`] for a specific URL path.
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

    /// Renders the page [`View`].
    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> PageViewStream<'cx>;

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

    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> PageViewStream<'cx> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// The async render function backing a [`PageFn`].
pub type PageRenderFn = for<'cx> fn(cx: &'cx Cx, body: Body) -> PageViewStream<'cx>;

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

    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> PageViewStream<'cx> {
        (self.render)(cx, body)
    }
}

pub type Slot<'cx> = PageViewStream<'cx>;

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

    /// Renders the layout, embedding the given child content
    /// [`Result`]`<`[`View`]`>` as its slot.
    fn render<'cx>(&'cx self, cx: &'cx Cx, slot: Slot<'cx>) -> PageViewStream<'cx>;
}

impl<L: Layout + ?Sized> Layout for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, slot: Slot<'cx>) -> PageViewStream<'cx> {
        (**self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// The async render function backing a [`LayoutFn`], receiving the rendered
/// child content as a [`Result`]`<`[`View`]`>`.
pub type LayoutRenderFn = for<'cx> fn(cx: &'cx Cx, slot: Slot) -> PageViewStream<'cx>;

/// A [`Layout`] backed by a plain render function.
///
/// Turns a function into a layout without implementing [`Layout`] on a
/// struct, pairing it with the path prefix it applies to.
#[derive(Debug, Clone)]
pub struct LayoutFn {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The async render function that wraps the child content [`Result`]`<`[`View`]`>`.
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

    fn render<'cx>(&self, cx: &'cx Cx, slot: Slot<'cx>) -> PageViewStream<'cx> {
        (self.render)(cx, slot)
    }
}

/// A [`Page`] paired with the [`Layout`]s that wrap it.
pub struct PageWithLayouts {
    page: Arc<dyn Page>,
    /// The matching layouts, ordered by ascending path length (outermost first).
    layouts: Vec<Arc<dyn Layout>>,
}

impl PageWithLayouts {
    /// Pairs `page` with the `layouts` that wrap it.
    ///
    /// `layouts` must be ordered from least- to most-specific (ascending path
    /// length); they are applied from the innermost (most specific) outward.
    #[must_use]
    pub fn new(page: Arc<dyn Page>, layouts: Vec<Arc<dyn Layout>>) -> Self {
        Self { page, layouts }
    }
}

impl Route for PageWithLayouts {
    fn id(&self) -> RouteId {
        self.page.id()
    }

    fn methods(&self) -> Methods<'_> {
        self.page.methods()
    }

    fn path(&self) -> &Path {
        self.page.path()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let stream = {
                let page = self.page.clone();
                let layouts = self.layouts.clone();
                let cx = cx.clone();
                // TODO: prevent inefficient cloning
                ViewStream::new(async move {
                    let mut slot = page.render(&cx, body);
                    for layout in layouts.iter().rev() {
                        slot = layout.render(&cx, slot);
                    }

                    topcoat_view::internal::forward(slot).await;
                })
            };

            ViewResponse::try_from(Box::pin(stream))
                .await?
                .into_response(cx)
        })
    }
}
