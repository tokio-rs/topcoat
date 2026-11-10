use std::borrow::Cow;

use topcoat_router::Router;

use crate::{Document, SecurityScheme, generate::generate};

/// The settings of an OpenAPI document, such as the API's title and version.
///
/// Call [`generate`](Self::generate) with a router to create the document
/// for its documented routes. For example, a route can serve it:
///
/// ```
/// use topcoat::{
///     Result,
///     context::Cx,
///     openapi::{Document, OpenApi},
///     router::{route, router},
/// };
///
/// #[route(GET "/openapi.json")]
/// async fn spec(cx: &Cx) -> Result<Document> {
///     Ok(OpenApi::new("Acme API", "1.0.0")
///         .description("Everything you need for tea.")
///         .generate(&router(cx)))
/// }
/// ```
///
/// See [`Operation`](crate::Operation) for documenting routes.
#[derive(Debug, Clone)]
pub struct OpenApi {
    pub(crate) title: Cow<'static, str>,
    pub(crate) version: Cow<'static, str>,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) servers: Vec<Cow<'static, str>>,
    pub(crate) tags: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    pub(crate) security_schemes: Vec<(Cow<'static, str>, SecurityScheme)>,
}

impl OpenApi {
    /// Creates the settings for an API with the given title and version.
    ///
    /// The version is that of the API itself, such as `1.0.0`.
    #[must_use]
    pub fn new(title: impl Into<Cow<'static, str>>, version: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: None,
            servers: Vec::new(),
            tags: Vec::new(),
            security_schemes: Vec::new(),
        }
    }

    /// Sets a description of the API as a whole.
    #[must_use]
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a base URL the API is reachable at, such as
    /// `https://api.example.com`.
    #[must_use]
    pub fn server(mut self, url: impl Into<Cow<'static, str>>) -> Self {
        self.servers.push(url.into());
        self
    }

    /// Describes a tag that operations use.
    ///
    /// Documentation tools show the description next to the tag's routes
    /// and list tags in the order they are described here.
    #[must_use]
    pub fn tag(
        mut self,
        name: impl Into<Cow<'static, str>>,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.tags.push((name.into(), description.into()));
        self
    }

    /// Registers a way for clients to authenticate under `name`.
    ///
    /// Operations require it by that name with
    /// [`Operation::security`](crate::Operation::security).
    #[must_use]
    pub fn security_scheme(
        mut self,
        name: impl Into<Cow<'static, str>>,
        scheme: SecurityScheme,
    ) -> Self {
        self.security_schemes.push((name.into(), scheme));
        self
    }

    /// Creates the document for the documented routes of `router`.
    ///
    /// A route is documented when an [`Operation`](crate::Operation) is
    /// attached to it, to a layer or layout that wraps it, or to the router
    /// builder, unless the operation is
    /// [hidden](crate::Operation::hidden).
    ///
    /// # Panics
    ///
    /// Panics when the routes are documented inconsistently:
    ///
    /// - An operation describes a path parameter that its route's path does not have.
    /// - An operation requires a security scheme that is not registered.
    /// - Two operations share an operation ID.
    /// - A type passed to [`Operation::query`](crate::Operation::query) is not a struct with named
    ///   fields.
    #[must_use]
    pub fn generate(&self, router: &Router) -> Document {
        generate(self, router)
    }
}
