use std::{any::Any, borrow::Cow};

use topcoat_core::context::{ContextMap, Cx};
use topcoat_view::{
    BoxView, Child,
    internal::{MoveView, ScopeView},
};

use crate::{
    Body, Handler, IntoPath, Methods, OwnedMethods, Path, RouteFuture, RouteId,
    response::AsyncIntoResponse, route, try_matched,
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

    /// The values attached to this page, such as metadata that describes it.
    ///
    /// Defaults to an empty map.
    fn context(&self) -> ContextMap {
        ContextMap::new()
    }

    /// Renders the page to a [`View`](topcoat_view::View) under the request
    /// context `cx`.
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a>;

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

    fn context(&self) -> ContextMap {
        (**self).context()
    }

    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Page);

/// The render function backing a [`PageFn`].
pub type PageRenderFn = for<'a> fn(cx: &'a Cx, body: Body) -> BoxView<'a>;

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
    /// The values attached to this page.
    context: ContextMap,
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
            context: ContextMap::new(),
        }
    }

    /// Adds `value` to the context of this page.
    ///
    /// The value shadows an earlier value of the same type. Read it during
    /// a request with [`context`](topcoat_core::context::context).
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
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

    fn context(&self) -> ContextMap {
        self.context.clone()
    }

    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> BoxView<'a> {
        (self.render)(cx, body)
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

    /// The values attached to this layout, such as defaults for the pages it
    /// wraps.
    ///
    /// Defaults to an empty map.
    fn context(&self) -> ContextMap {
        ContextMap::new()
    }

    /// Renders the layout, embedding the given child content [`Slot`], to a
    /// [`View`](topcoat_view::View) under the request context `cx`.
    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a>;
}

impl<L: Layout + ?Sized> Layout for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn context(&self) -> ContextMap {
        (**self).context()
    }

    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a> {
        (**self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// The render function backing a [`LayoutFn`], receiving the child content as
/// a [`Slot`].
pub type LayoutRenderFn = for<'a> fn(cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a>;

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
    /// The values attached to this layout.
    context: ContextMap,
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
            context: ContextMap::new(),
        }
    }

    /// Adds `value` to the context of this layout.
    ///
    /// The value shadows an earlier value of the same type. Read it during
    /// a request with [`context`](topcoat_core::context::context).
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
    }
}

impl Layout for LayoutFn {
    fn path(&self) -> &Path {
        &self.path
    }

    fn context(&self) -> ContextMap {
        self.context.clone()
    }

    fn render<'a>(&'a self, cx: &'a Cx, slot: Slot<'a>) -> BoxView<'a> {
        (self.render)(cx, slot)
    }
}

/// Selects the layouts in `layouts` wrapping a page at `path`: those whose
/// path is a prefix of `path`, ordered least- to most-specific so the
/// outermost layout comes first. Among layouts that share a path, the earlier
/// one in `layouts` is outside the later one.
///
/// Returns the positions of the selected layouts in `layouts`.
pub(crate) fn layouts_for_path(layouts: &[&dyn Layout], path: &Path) -> Vec<usize> {
    let mut matching: Vec<usize> = (0..layouts.len())
        .filter(|&layout| path.starts_with(layouts[layout].path()))
        .collect();
    matching.sort_by_key(|&layout| layouts[layout].path().len());
    matching
}

/// Composes `page` with `layouts`: the page is the innermost slot, each layout
/// wraps the slot beneath it, and the outermost layout is the view.
///
/// `layouts` must be ordered from least- to most-specific, outermost first.
fn render<'a>(
    page: &'a dyn Page,
    layouts: &'a [&'a dyn Layout],
    cx: &'a Cx,
    body: Body,
) -> BoxView<'a> {
    let mut view = page.render(cx, body);
    for &layout in layouts.iter().rev() {
        view = layout.render(cx, Slot::new(view));
    }
    view
}

/// Responds to a request that matched a page by rendering the page inside
/// its layouts.
///
/// # Panics
///
/// The response panics while rendering if the request of `cx` did not match
/// a page.
pub(crate) fn respond_with_page(cx: &Cx, body: Body) -> RouteFuture<'_> {
    // The response body outlives the handler, so the view owns a copy of the
    // request context and reads its page from it. It drives itself in place
    // as the outermost view of the build.
    let owned = cx.clone();
    Box::pin(async move {
        let view = MoveView::new(async move {
            let handler = try_matched(&owned)
                .and_then(|matched| matched.route)
                .map(|route| route.handler);
            let Some(Handler::Page { page, layouts }) = handler else {
                panic!("the request did not match a page");
            };
            let view = render(page, layouts, &owned, body);
            MoveView::drive(ScopeView::new(view)).await
        });
        view.async_into_response(cx).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A layout whose path is all a test cares about; it never renders in
    /// the selection tests.
    fn layout_at(path: &'static str) -> LayoutFn {
        LayoutFn::new(path, unreachable_layout)
    }

    /// Returns the positions in `layouts` that `layouts_for_path` selects
    /// for a page at `path`, in order.
    fn select(layouts: &[LayoutFn], path: &'static str) -> Vec<usize> {
        let layouts: Vec<&dyn Layout> =
            layouts.iter().map(|layout| layout as &dyn Layout).collect();
        layouts_for_path(&layouts, Path::new(path))
    }

    fn unreachable_layout<'a>(_cx: &'a Cx, _slot: Slot<'a>) -> BoxView<'a> {
        unreachable!("selection tests never render layouts")
    }

    /// Asserts that `layouts_for_path` selects the layouts at the `expected`
    /// paths, in order.
    fn assert_selects(layouts: &[LayoutFn], path: &'static str, expected: &[&'static str]) {
        let paths: Vec<&Path> = select(layouts, path)
            .into_iter()
            .map(|layout| layouts[layout].path())
            .collect();
        let expected: Vec<&Path> = expected.iter().copied().map(Path::new).collect();
        assert_eq!(paths, expected);
    }

    // -- layouts_for_path --

    #[test]
    fn for_path_orders_prefix_layouts_least_to_most_specific() {
        let layouts = [layout_at("/settings"), layout_at("/"), layout_at("/posts")];
        // The page at /settings/profile is wrapped by the root and /settings
        // layouts, in that order, regardless of registration order; the
        // /posts layout does not prefix it.
        assert_selects(&layouts, "/settings/profile", &["/", "/settings"]);
    }

    #[test]
    fn for_path_keeps_the_earlier_of_a_shared_path_outside() {
        let layouts = [layout_at("/admin"), layout_at("/admin")];
        assert_eq!(select(&layouts, "/admin/users"), [0, 1]);
    }

    #[test]
    fn for_path_rejects_partial_segments() {
        let layouts = [layout_at("/admin")];
        assert_eq!(select(&layouts, "/administrator"), [] as [usize; 0]);
    }
}
