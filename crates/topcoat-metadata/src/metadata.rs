mod open_graph;
mod robots;
mod twitter;

use std::borrow::Cow;

pub use open_graph::*;
pub use robots::*;
use topcoat_core::{
    context::{AsyncCxFn, BoxCxFn},
    error::Result,
};
pub use twitter::*;

use crate::{Field, Text, Url};

/// Information about a page, including its title and description.
///
/// Add it to a layout or page with `with(...)`. A page can replace individual
/// fields from its layouts. Fields it leaves unset come from the nearest
/// layout that supplies them:
///
/// ```
/// use topcoat::{
///     Result,
///     metadata::Metadata,
///     router::{Slot, layout, page},
///     view::{View, view},
/// };
///
/// #[layout("/", with(Metadata::new().description("Acme sells everything.")))]
/// async fn shell(slot: Slot<'_>) -> Result<impl View> {
///     Ok(view! { (slot) })
/// }
///
/// // Uses this title and inherits the description from the layout.
/// #[page("/products", with(Metadata::new().title("Products")))]
/// async fn products() -> Result<impl View> {
///     Ok(view! { <h1>"Products"</h1> })
/// }
/// ```
///
/// Use [`from_fn`](Self::from_fn) to load values for each request.
#[derive(Default)]
pub struct Metadata {
    pub(crate) title: Field<Text>,
    pub(crate) description: Field<Text>,
    pub(crate) canonical: Field<Url>,
    pub(crate) viewport: Field<Cow<'static, str>>,
    pub(crate) robots: Robots,
    pub(crate) theme_color: Field<Cow<'static, str>>,
    pub(crate) open_graph: Option<Box<OpenGraphFields>>,
    pub(crate) twitter: Option<Box<TwitterFields>>,
    /// Loads fields using the current request. Values set directly on this
    /// metadata override the loaded values.
    pub(crate) load: Option<BoxCxFn<Result<Metadata>>>,
}

impl Metadata {
    /// Creates empty metadata that inherits all its fields.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses `f` to load metadata for a request. For example, `f` can look up
    /// a product and use its name as the title.
    ///
    /// Set fields inside `f`. To override a loaded field for a particular
    /// page, set that field on the value returned by `from_fn`.
    ///
    /// Share `#[memoize]` data-loading functions with the page to avoid
    /// loading the same data twice.
    #[must_use]
    pub fn from_fn(f: impl AsyncCxFn<Result<Metadata>>) -> Self {
        Self {
            load: Some(BoxCxFn::new(f)),
            ..Self::default()
        }
    }

    /// Sets the page's title.
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.title = Field::Set(title.into());
        self
    }

    /// Sets a short summary of the page. Search engines may display it
    /// below the page title.
    #[must_use]
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        self.description = Field::Set(description.into());
        self
    }

    /// Omits the description, including any inherited value.
    #[must_use]
    pub fn no_description(mut self) -> Self {
        self.description = Field::Removed;
        self
    }

    /// Sets the preferred URL for this page. Search engines use this
    /// canonical URL when the same content is available at several URLs.
    #[must_use]
    pub fn canonical(mut self, canonical: impl Into<Url>) -> Self {
        self.canonical = Field::Set(canonical.into());
        self
    }

    /// Omits the canonical URL, including any inherited value.
    #[must_use]
    pub fn no_canonical(mut self) -> Self {
        self.canonical = Field::Removed;
        self
    }

    /// Tells mobile browsers how to size and scale the page.
    ///
    /// If no metadata sets a viewport, the default is
    /// `width=device-width, initial-scale=1`.
    #[must_use]
    pub fn viewport(mut self, viewport: impl Into<Cow<'static, str>>) -> Self {
        self.viewport = Field::Set(viewport.into());
        self
    }

    /// Omits the viewport tag, even when an inherited or default value exists.
    #[must_use]
    pub fn no_viewport(mut self) -> Self {
        self.viewport = Field::Removed;
        self
    }

    /// Sets the page's instructions for search engines through [`Robots`].
    /// For example, `.robots(|robots| robots.index(false))` asks them to
    /// leave the page out of search results.
    ///
    /// Each directive that `f` leaves unset is inherited separately.
    #[must_use]
    pub fn robots(mut self, f: impl FnOnce(RobotsBuilder) -> RobotsBuilder) -> Self {
        self.robots = f(RobotsBuilder(self.robots)).0;
        self
    }

    /// Suggests a color for the browser's interface around the page.
    /// For example, `.theme_color("#1d4ed8")`.
    #[must_use]
    pub fn theme_color(mut self, color: impl Into<Cow<'static, str>>) -> Self {
        self.theme_color = Field::Set(color.into());
        self
    }

    /// Omits the theme color, including any inherited value.
    #[must_use]
    pub fn no_theme_color(mut self) -> Self {
        self.theme_color = Field::Removed;
        self
    }

    /// Sets Open Graph metadata for social link previews.
    /// For example, `.open_graph(|og| og.site_name("Acme"))`.
    ///
    /// Unset fields are inherited. If no metadata sets an Open Graph title,
    /// description, or URL, the corresponding page value is used. Most
    /// pages only need to add preview images.
    #[must_use]
    pub fn open_graph(mut self, f: impl FnOnce(OpenGraphBuilder) -> OpenGraphBuilder) -> Self {
        let fields = self.open_graph.map(|fields| *fields).unwrap_or_default();
        self.open_graph = Some(Box::new(f(OpenGraphBuilder(fields)).0));
        self
    }

    /// Sets metadata for link previews on X.
    /// For example, `.twitter(|x| x.site("@acme"))`.
    ///
    /// X uses the [Open Graph fields](Self::open_graph) for the preview's
    /// title, description, and image. Fields left unset by `f` are inherited.
    #[must_use]
    pub fn twitter(mut self, f: impl FnOnce(TwitterBuilder) -> TwitterBuilder) -> Self {
        let fields = self.twitter.map(|fields| *fields).unwrap_or_default();
        self.twitter = Some(Box::new(f(TwitterBuilder(fields)).0));
        self
    }
}
