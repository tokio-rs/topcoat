use std::{borrow::Cow, collections::BTreeMap};

use schemars::Schema;
use serde::Serialize;
use serde_json::Value;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Method,
    content::Json,
    response::{IntoResponse, Response},
};

use crate::{SecurityScheme, parameter::ParameterLocation};

/// An OpenAPI document that describes the documented routes of a router.
///
/// Create it with [`OpenApi::generate`](crate::OpenApi::generate). Return it
/// from a route to serve it as JSON, or write it to a file with
/// [`to_json_pretty`](Self::to_json_pretty). It also implements
/// [`Serialize`] for other formats.
#[derive(Debug, Clone, Serialize)]
pub struct Document {
    pub(crate) openapi: &'static str,
    pub(crate) info: Info,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) servers: Vec<Server>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tags: Vec<Tag>,
    pub(crate) paths: BTreeMap<String, PathItem>,
    #[serde(skip_serializing_if = "Components::is_empty")]
    pub(crate) components: Components,
}

impl Document {
    /// Returns the document as compact JSON.
    ///
    /// # Panics
    ///
    /// Panics if a schema in the document cannot be written as JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("an OpenAPI document serializes to JSON")
    }

    /// Returns the document as indented JSON, which suits a file kept in
    /// version control.
    ///
    /// # Panics
    ///
    /// Panics if a schema in the document cannot be written as JSON.
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("an OpenAPI document serializes to JSON")
    }
}

/// Sends the document as JSON.
impl IntoResponse for &Document {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        Json(self).into_response(cx)
    }
}

/// Sends the document as JSON.
impl IntoResponse for Document {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (&self).into_response(cx)
    }
}

/// General information about the API.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Info {
    pub(crate) title: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) version: Cow<'static, str>,
}

/// A base URL the API is reachable at.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Server {
    pub(crate) url: Cow<'static, str>,
}

/// A description of a tag that operations use.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Tag {
    pub(crate) name: Cow<'static, str>,
    pub(crate) description: Cow<'static, str>,
}

/// The operations served at one path, by method.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    get: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    put: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    post: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    patch: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<OperationObject>,
}

impl PathItem {
    /// The methods a path item can describe.
    pub(crate) const METHODS: [Method; 8] = [
        Method::GET,
        Method::PUT,
        Method::POST,
        Method::DELETE,
        Method::OPTIONS,
        Method::HEAD,
        Method::PATCH,
        Method::TRACE,
    ];

    /// Returns whether a path item can describe an operation for `method`.
    pub(crate) fn describes(method: &Method) -> bool {
        Self::METHODS.contains(method)
    }

    /// Returns the place of the operation served for `method`, or `None` for
    /// a method that a path item cannot describe.
    pub(crate) fn operation_mut(
        &mut self,
        method: &Method,
    ) -> Option<&mut Option<OperationObject>> {
        match method.as_str() {
            "GET" => Some(&mut self.get),
            "PUT" => Some(&mut self.put),
            "POST" => Some(&mut self.post),
            "DELETE" => Some(&mut self.delete),
            "OPTIONS" => Some(&mut self.options),
            "HEAD" => Some(&mut self.head),
            "PATCH" => Some(&mut self.patch),
            "TRACE" => Some(&mut self.trace),
            _ => None,
        }
    }
}

/// One operation: a method served at a path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperationObject {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tags: Vec<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_id: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) parameters: Vec<ParameterObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_body: Option<RequestBodyObject>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) responses: BTreeMap<u16, ResponseObject>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) deprecated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) security: Option<Vec<SecurityRequirement>>,
}

/// One accepted way to authenticate: the name of a security scheme, mapped
/// to the scopes it needs.
pub(crate) type SecurityRequirement = BTreeMap<Cow<'static, str>, Vec<Cow<'static, str>>>;

/// A parameter of an operation.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ParameterObject {
    pub(crate) name: Cow<'static, str>,
    #[serde(rename = "in")]
    pub(crate) location: ParameterLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<Cow<'static, str>>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) required: bool,
    pub(crate) schema: Schema,
}

/// The request body of an operation.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RequestBodyObject {
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) required: bool,
    pub(crate) content: BTreeMap<&'static str, MediaType>,
}

/// One response of an operation.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResponseObject {
    pub(crate) description: Cow<'static, str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) content: BTreeMap<&'static str, MediaType>,
}

/// The payload of a body in one media type.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MediaType {
    pub(crate) schema: Schema,
}

/// The definitions that the rest of the document refers to.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Components {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) schemas: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) security_schemes: BTreeMap<Cow<'static, str>, SecurityScheme>,
}

impl Components {
    /// Returns whether there is nothing to list.
    fn is_empty(&self) -> bool {
        self.schemas.is_empty() && self.security_schemes.is_empty()
    }
}

/// Lets serde skip a flag that is not set.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(flag: &bool) -> bool {
    !flag
}
