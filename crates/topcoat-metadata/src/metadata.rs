mod robots;

use std::borrow::Cow;

pub use robots::*;
use topcoat_core::{
    context::{AsyncCxFn, BoxCxFn},
    error::Result,
};

use crate::{Text, Url};

/// The metadata of a page, such as its title and description.
///
/// Attach it to a layout or a page with `with(...)`. Each one sets only the
/// fields it cares about, and a page reads every field from the closest
/// level that sets it:
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
/// // Sets its own title and keeps the layout's description.
/// #[page("/products", with(Metadata::new().title("Products")))]
/// async fn products() -> Result<impl View> {
///     Ok(view! { <h1>"Products"</h1> })
/// }
/// ```
///
/// Use [`from_fn`](Self::from_fn) for metadata that depends on the request.
#[derive(Default)]
pub struct Metadata {
    pub(crate) title: Field<Text>,
    pub(crate) description: Field<Text>,
    pub(crate) canonical: Field<Url>,
    pub(crate) viewport: Field<Cow<'static, str>>,
    pub(crate) robots: Robots,
    /// Loads more fields for each request. Fields set on this value take
    /// precedence over the loaded ones.
    pub(crate) load: Option<BoxCxFn<Result<Metadata>>>,
}

impl Metadata {
    /// Creates metadata that sets no fields.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates metadata that `f` loads for each request, such as a title
    /// showing a product's name.
    ///
    /// Set every field inside `f`. A field set on the returned value
    /// overrides the one `f` loads, which lets you adjust shared metadata
    /// for one page.
    ///
    /// Load data through `#[memoize]` functions that the page uses too, so
    /// both share one load.
    #[must_use]
    pub fn from_fn(f: impl AsyncCxFn<Result<Metadata>>) -> Self {
        Self {
            load: Some(BoxCxFn::new(f)),
            ..Self::default()
        }
    }

    /// Sets the title of the page.
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.title = Field::Set(title.into());
        self
    }

    /// Sets the description of the page, which search engines may show
    /// below its title.
    #[must_use]
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        self.description = Field::Set(description.into());
        self
    }

    /// Removes the description set further up, so the page has none.
    #[must_use]
    pub fn no_description(mut self) -> Self {
        self.description = Field::Removed;
        self
    }

    /// Sets the canonical URL of the page: the URL search engines should
    /// treat as the original when the same content is reachable at several
    /// URLs.
    #[must_use]
    pub fn canonical(mut self, canonical: impl Into<Url>) -> Self {
        self.canonical = Field::Set(canonical.into());
        self
    }

    /// Removes the canonical URL set further up, so the page has none.
    #[must_use]
    pub fn no_canonical(mut self) -> Self {
        self.canonical = Field::Removed;
        self
    }

    /// Sets the viewport of the page, which tells mobile browsers how to
    /// size and scale it.
    ///
    /// Defaults to `width=device-width, initial-scale=1` when no level sets
    /// it.
    #[must_use]
    pub fn viewport(mut self, viewport: impl Into<Cow<'static, str>>) -> Self {
        self.viewport = Field::Set(viewport.into());
        self
    }

    /// Removes the viewport, including the default, so the page has none.
    #[must_use]
    pub fn no_viewport(mut self) -> Self {
        self.viewport = Field::Removed;
        self
    }

    /// Sets the [`Robots`] directives of the page, as in
    /// `.robots(|robots| robots.index(false))` to keep it out of search
    /// results.
    ///
    /// Directives that `f` leaves unset come from further up.
    #[must_use]
    pub fn robots(mut self, f: impl FnOnce(RobotsBuilder) -> RobotsBuilder) -> Self {
        self.robots = f(RobotsBuilder(self.robots)).0;
        self
    }
}

/// The state of one field in one level of metadata.
#[derive(Default)]
pub(crate) enum Field<T> {
    /// The field is not set here, so a level further up decides.
    #[default]
    Inherit,
    /// The field is set to this value.
    Set(T),
    /// The field is removed, hiding any value set further up.
    Removed,
}

impl<T> Field<T> {
    /// Replaces this field with `field` if it is not set or removed yet.
    pub(crate) fn fill(&mut self, field: Field<T>) {
        if let Self::Inherit = self {
            *self = field;
        }
    }

    /// Returns the value if the field is set.
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Inherit | Self::Removed => None,
        }
    }

    /// Converts the value with `f`, keeping the state.
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Field<U> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(f(value)),
            Self::Removed => Field::Removed,
        }
    }

    /// Borrows the value, keeping the state.
    pub(crate) fn as_ref(&self) -> Field<&T> {
        match self {
            Self::Inherit => Field::Inherit,
            Self::Set(value) => Field::Set(value),
            Self::Removed => Field::Removed,
        }
    }
}
