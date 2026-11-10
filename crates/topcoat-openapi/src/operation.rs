use std::{any::type_name, borrow::Cow, collections::BTreeMap};

use schemars::JsonSchema;
use topcoat_router::StatusCode;

use crate::{
    Content, Parameter,
    content::{ContentType, SchemaFn},
};

/// Describes a route in the API document.
///
/// Add it to a route with `with(...)`. A route appears in the document when
/// an `Operation` is attached to it or to a layer that wraps it:
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     context::Cx,
///     openapi::{Operation, schemars::JsonSchema},
///     router::{Body, Next, StatusCode, content::Json, layer, response::Response, route},
/// };
///
/// #[derive(Deserialize, JsonSchema)]
/// struct CreateUser {
///     name: String,
/// }
///
/// #[derive(Serialize, JsonSchema)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// // Every route under `/api` is documented and tagged.
/// #[layer("/api", with(Operation::new().tag("api")))]
/// async fn api(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
///     next.run(cx, body).await
/// }
///
/// #[route(POST "/api/users", with(
///     Operation::new()
///         .summary("Create a user")
///         .request::<Json<CreateUser>>()
///         .response_with::<Json<User>>(StatusCode::CREATED, "The new user.")
/// ))]
/// async fn create_user(
///     cx: &Cx,
///     Json(input): Json<CreateUser>,
/// ) -> Result<(StatusCode, Json<User>)> {
///     Ok((
///         StatusCode::CREATED,
///         Json(User {
///             id: 1,
///             name: input.name,
///         }),
///     ))
/// }
/// ```
///
/// A route combines its own operation with those of its layers, and the
/// closest value of each setting takes priority. Tags, parameters, and
/// responses add up instead: a route keeps the ones its layers declare and
/// replaces only a parameter or response status it declares itself.
#[derive(Debug, Clone, Default)]
pub struct Operation {
    pub(crate) summary: Option<Cow<'static, str>>,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) id: Option<Cow<'static, str>>,
    pub(crate) deprecated: Option<bool>,
    pub(crate) hidden: Option<bool>,
    pub(crate) tags: Vec<Cow<'static, str>>,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) queries: Vec<QueryType>,
    pub(crate) request: Option<ContentType>,
    pub(crate) responses: BTreeMap<u16, ResponseType>,
    pub(crate) security: Option<Vec<Cow<'static, str>>>,
}

impl Operation {
    /// Creates an operation that inherits every setting.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a short summary of what the route does.
    #[must_use]
    pub fn summary(mut self, summary: impl Into<Cow<'static, str>>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Sets a longer explanation of the route's behavior.
    #[must_use]
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the name that identifies the operation, which client generators
    /// use to name their functions.
    ///
    /// An ID has to be unique in the document, so set it on a route that
    /// serves a single method rather than on a layer.
    #[must_use]
    pub fn operation_id(mut self, operation_id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(operation_id.into());
        self
    }

    /// Marks the route as deprecated, so clients know to move away from it.
    #[must_use]
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.deprecated = Some(deprecated);
        self
    }

    /// Chooses whether to leave the route out of the document.
    ///
    /// Pass `true` to hide a route that a layer documents. Pass `false` to
    /// show a route below a layer that hides its routes.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = Some(hidden);
        self
    }

    /// Adds a tag, which documentation tools use to group routes.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<Cow<'static, str>>) -> Self {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
        self
    }

    /// Describes a path, query, header, or cookie parameter.
    ///
    /// Describing the same parameter again replaces the earlier entry.
    #[must_use]
    pub fn parameter(mut self, parameter: Parameter) -> Self {
        self.parameters.retain(|other| !other.same_as(&parameter));
        self.parameters.push(parameter);
        self
    }

    /// Documents every field of the struct `T` as a query parameter.
    ///
    /// This suits a struct read with `query_params`. Optional fields become
    /// optional parameters, and doc comments on fields become their
    /// descriptions:
    ///
    /// ```
    /// use topcoat::{
    ///     openapi::{Operation, schemars::JsonSchema},
    ///     router::query_params,
    /// };
    ///
    /// #[query_params]
    /// #[derive(JsonSchema)]
    /// struct PostsQuery {
    ///     /// The page to show, starting at 1.
    ///     page: Option<u32>,
    /// }
    ///
    /// let operation = Operation::new().query::<PostsQuery>();
    /// ```
    #[must_use]
    pub fn query<T: JsonSchema>(mut self) -> Self {
        self.queries.push(QueryType {
            name: type_name::<T>(),
            schema: T::json_schema,
        });
        self
    }

    /// Documents the request body, for example
    /// `.request::<Json<CreateUser>>()`.
    ///
    /// Wrap the type in [`Option`] for a body that requests may leave out.
    #[must_use]
    pub fn request<T: Content>(mut self) -> Self {
        self.request = Some(ContentType::of::<T>());
        self
    }

    /// Documents a response without a body, for example
    /// `.response(StatusCode::NO_CONTENT, "The user was deleted.")`.
    ///
    /// Declaring a status again replaces its earlier response.
    #[must_use]
    pub fn response(
        mut self,
        status: StatusCode,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.responses.insert(
            status.as_u16(),
            ResponseType {
                description: description.into(),
                content: None,
            },
        );
        self
    }

    /// Documents a response with a body, for example
    /// `.response_with::<Json<User>>(StatusCode::OK, "The user.")`.
    ///
    /// Declaring a status again replaces its earlier response.
    #[must_use]
    pub fn response_with<T: Content>(
        mut self,
        status: StatusCode,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.responses.insert(
            status.as_u16(),
            ResponseType {
                description: description.into(),
                content: Some(ContentType::of::<T>()),
            },
        );
        self
    }

    /// Requires clients to authenticate with the security scheme registered
    /// under `scheme`.
    ///
    /// Calling this again adds an alternative, so clients may use either
    /// scheme. The requirements of an operation replace those of its layers
    /// as a whole.
    ///
    /// See [`OpenApi::security_scheme`](crate::OpenApi::security_scheme) for
    /// registering a scheme.
    #[must_use]
    pub fn security(mut self, scheme: impl Into<Cow<'static, str>>) -> Self {
        self.security.get_or_insert_default().push(scheme.into());
        self
    }

    /// Documents the route as usable without authentication, even when a
    /// layer requires it.
    #[must_use]
    pub fn no_security(mut self) -> Self {
        self.security = Some(Vec::new());
        self
    }

    /// Combines the operations attached to a route into one. Pass them
    /// closest first, so the route's own operation precedes those of its
    /// layers. Returns `None` when there are none.
    pub(crate) fn combine<'a>(levels: impl IntoIterator<Item = &'a Operation>) -> Option<Self> {
        let mut levels = levels.into_iter();
        let mut combined = levels.next()?.clone();
        for outer in levels {
            combined.inherit(outer);
        }
        Some(combined)
    }

    /// Fills the settings this operation leaves unset from `outer`, and
    /// places the tags, parameters, and responses of `outer` before its own.
    fn inherit(&mut self, outer: &Operation) {
        fn fill<T: Clone>(field: &mut Option<T>, outer: Option<&T>) {
            if field.is_none() {
                *field = outer.cloned();
            }
        }

        fill(&mut self.summary, outer.summary.as_ref());
        fill(&mut self.description, outer.description.as_ref());
        fill(&mut self.id, outer.id.as_ref());
        fill(&mut self.deprecated, outer.deprecated.as_ref());
        fill(&mut self.hidden, outer.hidden.as_ref());
        fill(&mut self.request, outer.request.as_ref());
        fill(&mut self.security, outer.security.as_ref());

        let mut tags = outer.tags.clone();
        for tag in self.tags.drain(..) {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        self.tags = tags;

        let mut parameters: Vec<Parameter> = outer
            .parameters
            .iter()
            .filter(|parameter| !self.parameters.iter().any(|own| own.same_as(parameter)))
            .cloned()
            .collect();
        parameters.append(&mut self.parameters);
        self.parameters = parameters;

        let mut queries = outer.queries.clone();
        queries.append(&mut self.queries);
        self.queries = queries;

        for (status, response) in &outer.responses {
            self.responses
                .entry(*status)
                .or_insert_with(|| response.clone());
        }
    }
}

/// A struct whose fields are documented as query parameters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QueryType {
    /// The Rust name of the struct, for error messages.
    pub(crate) name: &'static str,
    pub(crate) schema: SchemaFn,
}

/// A documented response to a route.
#[derive(Debug, Clone)]
pub(crate) struct ResponseType {
    pub(crate) description: Cow<'static, str>,
    pub(crate) content: Option<ContentType>,
}

#[cfg(test)]
mod tests {
    use topcoat_router::content::Json;

    use super::*;

    /// Combines operations given closest first.
    fn combine(levels: &[Operation]) -> Operation {
        Operation::combine(levels).unwrap()
    }

    #[test]
    fn combining_nothing_yields_no_operation() {
        assert!(Operation::combine(&[]).is_none());
    }

    #[test]
    fn the_closest_value_of_a_setting_takes_priority() {
        let layer = Operation::new()
            .summary("Layer")
            .description("From the layer");
        let route = Operation::new().summary("Route");

        let combined = combine(&[route, layer]);

        assert_eq!(combined.summary.as_deref(), Some("Route"));
        assert_eq!(combined.description.as_deref(), Some("From the layer"));
    }

    #[test]
    fn a_route_can_show_itself_below_a_layer_that_hides_its_routes() {
        let layer = Operation::new().hidden(true);

        assert_eq!(
            combine(&[Operation::new(), layer.clone()]).hidden,
            Some(true)
        );
        assert_eq!(
            combine(&[Operation::new().hidden(false), layer]).hidden,
            Some(false)
        );
    }

    #[test]
    fn tags_add_up_from_the_outermost_level_without_repeating() {
        let app = Operation::new().tag("api");
        let layer = Operation::new().tag("users");
        let route = Operation::new().tag("api").tag("admin");

        let combined = combine(&[route, layer, app]);

        assert_eq!(combined.tags, ["api", "users", "admin"]);
    }

    #[test]
    fn responses_add_up_and_the_closest_one_wins_per_status() {
        let layer = Operation::new()
            .response(StatusCode::UNAUTHORIZED, "Not signed in.")
            .response(StatusCode::NOT_FOUND, "Layer");
        let route = Operation::new()
            .response(StatusCode::NOT_FOUND, "Route")
            .response_with::<Json<String>>(StatusCode::OK, "Found.");

        let combined = combine(&[route, layer]);

        let statuses: Vec<u16> = combined.responses.keys().copied().collect();
        assert_eq!(statuses, [200, 401, 404]);
        assert_eq!(combined.responses[&404].description, "Route");
        assert!(combined.responses[&200].content.is_some());
        assert!(combined.responses[&401].content.is_none());
    }

    #[test]
    fn parameters_add_up_and_the_closest_one_wins_per_name_and_location() {
        let layer = Operation::new()
            .parameter(Parameter::header("x-tenant").description("Layer"))
            .parameter(Parameter::query("page"));
        let route = Operation::new()
            .parameter(Parameter::header("x-tenant").description("Route"))
            // Shares its name with the query parameter, but not its location.
            .parameter(Parameter::cookie("page"));

        let combined = combine(&[route, layer]);

        let described: Vec<_> = combined
            .parameters
            .iter()
            .map(|parameter| {
                (
                    parameter.location,
                    &*parameter.name,
                    parameter.description.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            described,
            [
                (crate::parameter::ParameterLocation::Query, "page", None),
                (
                    crate::parameter::ParameterLocation::Header,
                    "x-tenant",
                    Some("Route")
                ),
                (crate::parameter::ParameterLocation::Cookie, "page", None),
            ]
        );
    }

    #[test]
    fn describing_a_parameter_again_replaces_it() {
        let operation = Operation::new()
            .parameter(Parameter::query("page").description("First"))
            .parameter(Parameter::query("page").description("Second"));

        assert_eq!(operation.parameters.len(), 1);
        assert_eq!(
            operation.parameters[0].description.as_deref(),
            Some("Second")
        );
    }

    #[test]
    fn security_requirements_replace_those_of_the_layers_as_a_whole() {
        let layer = Operation::new().security("token").security("key");

        let inherited = combine(&[Operation::new(), layer.clone()]);
        assert_eq!(inherited.security.unwrap(), ["token", "key"]);

        let replaced = combine(&[Operation::new().security("session"), layer.clone()]);
        assert_eq!(replaced.security.unwrap(), ["session"]);

        let public = combine(&[Operation::new().no_security(), layer]);
        assert!(public.security.unwrap().is_empty());
    }
}
