use std::borrow::Cow;

use schemars::JsonSchema;
use serde::Serialize;

use crate::content::SchemaFn;

/// A single request parameter: a path segment, a query key, a header, or a
/// cookie.
///
/// Pass it to [`Operation::parameter`](crate::Operation::parameter). The
/// value is documented as a string unless [`schema`](Self::schema) names
/// another type:
///
/// ```
/// use topcoat::openapi::{Operation, Parameter};
///
/// let operation = Operation::new().parameter(
///     Parameter::path("post_id")
///         .schema::<u64>()
///         .description("The post to load."),
/// );
/// ```
///
/// Path parameters are documented even without an entry, because they are
/// part of the route's path. Add one to describe the parameter or to give
/// it a type.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub(crate) location: ParameterLocation,
    pub(crate) name: Cow<'static, str>,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) required: bool,
    pub(crate) schema: Option<SchemaFn>,
}

impl Parameter {
    /// Creates a parameter at `location` that is not required.
    fn new(location: ParameterLocation, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            location,
            name: name.into(),
            description: None,
            required: false,
            schema: None,
        }
    }

    /// Describes the path parameter `name`, written `{name}` in the route's
    /// path. A path parameter is always required.
    #[must_use]
    pub fn path(name: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ParameterLocation::Path, name)
    }

    /// Describes the query string key `name`.
    #[must_use]
    pub fn query(name: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ParameterLocation::Query, name)
    }

    /// Describes the request header `name`.
    #[must_use]
    pub fn header(name: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ParameterLocation::Header, name)
    }

    /// Describes the cookie `name`.
    #[must_use]
    pub fn cookie(name: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ParameterLocation::Cookie, name)
    }

    /// Explains what the parameter is for.
    #[must_use]
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Chooses whether a request has to send the parameter. Parameters are
    /// optional by default, except path parameters, which are always
    /// required.
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Documents the parameter's value with the schema of `T`, for example
    /// `.schema::<u64>()` for a numeric ID.
    #[must_use]
    pub fn schema<T: JsonSchema>(mut self) -> Self {
        self.schema = Some(|generator| generator.subschema_for::<T>());
        self
    }

    /// Returns whether `other` describes the same parameter of a request.
    pub(crate) fn same_as(&self, other: &Parameter) -> bool {
        self.location == other.location && self.name == other.name
    }
}

/// The part of a request that carries a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}
