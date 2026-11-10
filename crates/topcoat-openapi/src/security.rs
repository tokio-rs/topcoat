use std::borrow::Cow;

use serde::Serialize;

/// A way for clients to authenticate, such as a bearer token or an API key.
///
/// Register it under a name with
/// [`OpenApi::security_scheme`](crate::OpenApi::security_scheme), then
/// require it with [`Operation::security`](crate::Operation::security):
///
/// ```
/// use topcoat::openapi::{OpenApi, Operation, SecurityScheme};
///
/// let api = OpenApi::new("Acme API", "1.0.0")
///     .security_scheme("token", SecurityScheme::bearer().bearer_format("JWT"));
/// let operation = Operation::new().security("token");
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<Cow<'static, str>>,
    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    location: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bearer_format: Option<Cow<'static, str>>,
}

impl SecurityScheme {
    /// Creates a scheme that uses the HTTP `Authorization` header with the
    /// given authentication scheme.
    fn http(scheme: &'static str) -> Self {
        Self {
            kind: "http",
            description: None,
            name: None,
            location: None,
            scheme: Some(scheme),
            bearer_format: None,
        }
    }

    /// Creates a scheme that sends an API key under `name` in `location`.
    fn api_key(location: &'static str, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: "apiKey",
            description: None,
            name: Some(name.into()),
            location: Some(location),
            scheme: None,
            bearer_format: None,
        }
    }

    /// Authenticates with a token sent as `Authorization: Bearer <token>`.
    #[must_use]
    pub fn bearer() -> Self {
        Self::http("bearer")
    }

    /// Authenticates with a user name and password sent through HTTP basic
    /// authentication.
    #[must_use]
    pub fn basic() -> Self {
        Self::http("basic")
    }

    /// Authenticates with an API key sent in the request header `name`.
    #[must_use]
    pub fn api_key_header(name: impl Into<Cow<'static, str>>) -> Self {
        Self::api_key("header", name)
    }

    /// Authenticates with an API key sent as the query string key `name`.
    #[must_use]
    pub fn api_key_query(name: impl Into<Cow<'static, str>>) -> Self {
        Self::api_key("query", name)
    }

    /// Authenticates with an API key sent in the cookie `name`.
    #[must_use]
    pub fn api_key_cookie(name: impl Into<Cow<'static, str>>) -> Self {
        Self::api_key("cookie", name)
    }

    /// Explains how clients obtain and use the credentials.
    #[must_use]
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Names the format of a bearer token, such as `JWT`. This is a hint for
    /// readers and only applies to a [`bearer`](Self::bearer) scheme.
    #[must_use]
    pub fn bearer_format(mut self, format: impl Into<Cow<'static, str>>) -> Self {
        self.bearer_format = Some(format.into());
        self
    }
}
