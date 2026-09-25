use http::{
    StatusCode,
    header::{CONTENT_TYPE, HeaderValue},
};
use serde::{Deserialize, Serialize};
use topcoat_core::{context::Cx, error::Result};

use super::validate::{Validate, ValidateWithCx};
use crate::response::{IntoResponse, Response};

/// A single field validation failure.
///
/// The field is a dotted path into the input (`"address.zip"`), the message
/// is client-safe text shown next to the field, and the optional code is a
/// stable machine-readable tag (`"required"`) for API clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    field: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl ValidationError {
    /// Builds an error for `field` with a client-safe `message` and no code.
    #[must_use]
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: None,
        }
    }

    /// Attaches a stable machine-readable `code` (`"required"`).
    ///
    /// Prefer the canonical [`rules`](super::rules) codes so clients can match
    /// on them across endpoints.
    ///
    /// ```rust
    /// use topcoat::validation::ValidationError;
    ///
    /// let error = ValidationError::new("email", "email is required").with_code("required");
    /// assert_eq!(error.code(), Some("required"));
    /// ```
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Returns the dotted path of the field that failed validation.
    #[must_use]
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Returns the client-safe description of the failure.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the machine-readable code, if one was attached.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// The collected field failures of one validation pass.
///
/// Build one inside a [`Validate`](super::Validate) impl with [`add`](Self::add)
/// and [`check`](Self::check), then finish with [`into_result`](Self::into_result).
/// An empty collection validates successfully; a non-empty one becomes the
/// handler error and renders as `422 Unprocessable Entity`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    /// Builds an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Pushes a failure for `field` with a client-safe `message`.
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.0.push(ValidationError::new(field, message));
        self
    }

    /// Pushes a failure for `field` with a `code` and a client-safe `message`.
    pub fn add_with_code(
        &mut self,
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        self.0
            .push(ValidationError::new(field, message).with_code(code));
        self
    }

    /// Pushes a prebuilt failure.
    pub fn add_error(&mut self, error: ValidationError) -> &mut Self {
        self.0.push(error);
        self
    }

    /// Pushes a failure for `field` unless `ok` holds.
    ///
    /// This is the tersest way to write a rule inside a validate impl. The
    /// condition reads as the valid case, the message as the failure.
    ///
    /// ```rust
    /// use topcoat::validation::ValidationErrors;
    ///
    /// let mut errors = ValidationErrors::new();
    /// errors.check("title", false, "title cannot be empty");
    /// assert!(errors.has_field("title"));
    /// ```
    pub fn check(
        &mut self,
        field: impl Into<String>,
        ok: bool,
        message: impl Into<String>,
    ) -> &mut Self {
        if !ok {
            self.0.push(ValidationError::new(field, message));
        }
        self
    }

    /// Pushes a coded failure for `field` unless `ok` holds.
    ///
    /// This is the coded form of [`check`](Self::check). Reach for the
    /// single-bound [`rules`](super::rules) (`min_length`, `max_length`)
    /// when codes matter: a combined bound like `length` cannot tell which
    /// side failed.
    ///
    /// ```rust
    /// use topcoat::validation::{ValidationErrors, rules};
    ///
    /// let mut errors = ValidationErrors::new();
    /// errors.check_with_code("title", false, "required", "title cannot be empty");
    /// assert_eq!(errors.first_message("title"), Some("title cannot be empty"));
    /// ```
    pub fn check_with_code(
        &mut self,
        field: impl Into<String>,
        ok: bool,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        if !ok {
            self.0
                .push(ValidationError::new(field, message).with_code(code));
        }
        self
    }

    /// Validates a nested value, prefixing its failures with `field`.
    ///
    /// Inner paths join with dots (`"address"` plus `"zip"` becomes
    /// `"address.zip"`), so nested failures stay addressable with
    /// [`field`](Self::field) and [`first_message`](Self::first_message).
    /// A value without failures appends nothing. Nesting composes: an inner
    /// value that nests itself keeps its full path.
    ///
    /// Nested values arrive through JSON bodies, where objects nest. HTML
    /// forms post flat pairs, so form inputs stay flat structs.
    ///
    /// ```rust
    /// use topcoat::validation::{Validate, ValidationErrors};
    ///
    /// # struct Address;
    /// # impl Validate for Address {
    /// #     fn validate(&self) -> Result<(), ValidationErrors> {
    /// #         let mut errors = ValidationErrors::new();
    /// #         errors.add("zip", "zip cannot be empty");
    /// #         errors.into_result()
    /// #     }
    /// # }
    /// # struct Order { address: Address }
    /// # impl Validate for Order {
    /// #     fn validate(&self) -> Result<(), ValidationErrors> {
    /// #         let mut errors = ValidationErrors::new();
    /// errors.nest("address", &self.address);
    /// #         errors.into_result()
    /// #     }
    /// # }
    /// ```
    pub fn nest<T: Validate + ?Sized>(&mut self, field: &str, value: &T) -> &mut Self {
        if let Err(inner) = value.validate() {
            for error in inner {
                self.0.push(join_error(field, None, &error));
            }
        }
        self
    }

    /// Validates each nested value, prefixing failures with `field` and the index.
    ///
    /// Paths read `"items[0].name"`, composing with [`nest`](Self::nest)
    /// when an item nests itself. An empty slice appends nothing.
    ///
    /// ```rust
    /// use topcoat::validation::{Validate, ValidationErrors};
    ///
    /// # struct Item;
    /// # impl Validate for Item {
    /// #     fn validate(&self) -> Result<(), ValidationErrors> {
    /// #         let mut errors = ValidationErrors::new();
    /// #         errors.add("name", "name cannot be empty");
    /// #         errors.into_result()
    /// #     }
    /// # }
    /// let mut errors = ValidationErrors::new();
    /// errors.nest_each("items", &[Item, Item]);
    /// assert!(errors.has_field("items[0].name"));
    /// assert!(errors.has_field("items[1].name"));
    /// ```
    pub fn nest_each<T: Validate>(&mut self, field: &str, values: &[T]) -> &mut Self {
        for (index, value) in values.iter().enumerate() {
            if let Err(inner) = value.validate() {
                for error in inner {
                    self.0.push(join_error(field, Some(index), &error));
                }
            }
        }
        self
    }

    /// Validates a nested value against its context rules, prefixing failures.
    ///
    /// The async form of [`nest`](Self::nest): runs the value's
    /// [`ValidateWithCx`](super::validate::ValidateWithCx) impl and joins paths
    /// the same way, so sync and async nesting stay addressable alike.
    ///
    /// ```rust,no_run
    /// # use topcoat::{
    /// #     Result,
    /// #     context::Cx,
    /// #     validation::{ValidateWithCx, ValidationErrors},
    /// # };
    /// # struct Member;
    /// # impl ValidateWithCx for Member {
    /// #     async fn validate_cx(&self, _cx: &Cx) -> Result<(), ValidationErrors> {
    /// #         ValidationErrors::new().into_result()
    /// #     }
    /// # }
    /// # async fn run(cx: &Cx, member: &Member) {
    /// let mut errors = ValidationErrors::new();
    /// errors.nest_cx("member", member, cx).await;
    /// assert!(errors.is_empty());
    /// # }
    /// ```
    pub async fn nest_cx<T: ValidateWithCx + ?Sized>(
        &mut self,
        field: &str,
        value: &T,
        cx: &Cx,
    ) -> &mut Self {
        if let Err(inner) = value.validate_cx(cx).await {
            for error in inner {
                self.0.push(join_error(field, None, &error));
            }
        }
        self
    }

    /// Validates each nested value against its context rules, prefixing with index.
    ///
    /// The async form of [`nest_each`](Self::nest_each): paths read
    /// `"items[0].name"`, composing with [`nest_cx`](Self::nest_cx) when an
    /// item nests itself.
    ///
    /// ```rust,no_run
    /// # use topcoat::{
    /// #     Result,
    /// #     context::Cx,
    /// #     validation::{ValidateWithCx, ValidationErrors},
    /// # };
    /// # struct Member;
    /// # impl ValidateWithCx for Member {
    /// #     async fn validate_cx(&self, _cx: &Cx) -> Result<(), ValidationErrors> {
    /// #         ValidationErrors::new().into_result()
    /// #     }
    /// # }
    /// # async fn run(cx: &Cx, members: &[Member]) {
    /// let mut errors = ValidationErrors::new();
    /// errors.nest_each_cx("members", members, cx).await;
    /// assert!(errors.is_empty());
    /// # }
    /// ```
    pub async fn nest_each_cx<T: ValidateWithCx>(
        &mut self,
        field: &str,
        values: &[T],
        cx: &Cx,
    ) -> &mut Self {
        for (index, value) in values.iter().enumerate() {
            if let Err(inner) = value.validate_cx(cx).await {
                for error in inner {
                    self.0.push(join_error(field, Some(index), &error));
                }
            }
        }
        self
    }

    /// Returns whether no failure was collected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of collected failures.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the collected failures in insertion order.
    #[must_use]
    pub fn errors(&self) -> &[ValidationError] {
        &self.0
    }

    /// Consumes the collection into its failures in insertion order.
    #[must_use]
    pub fn into_inner(self) -> Vec<ValidationError> {
        self.0
    }

    /// Iterates over the collected failures in insertion order.
    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &ValidationError> + ExactSizeIterator {
        self.0.iter()
    }

    /// Iterates over the failures of one field in insertion order.
    #[must_use]
    pub fn field(&self, field: &str) -> impl DoubleEndedIterator<Item = &ValidationError> {
        self.0.iter().filter(move |error| error.field() == field)
    }

    /// Returns whether any failure targets `field`.
    #[must_use]
    pub fn has_field(&self, field: &str) -> bool {
        self.0.iter().any(|error| error.field() == field)
    }

    /// Iterates over the messages of one field in insertion order.
    #[must_use]
    pub fn messages(&self, field: &str) -> impl DoubleEndedIterator<Item = &str> {
        self.field(field).map(ValidationError::message)
    }

    /// Returns the first message of one field, for inline form rendering.
    #[must_use]
    pub fn first_message(&self, field: &str) -> Option<&str> {
        self.messages(field).next()
    }

    /// Finishes the pass: `Ok(())` when empty, `Err(self)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns the collected failures when at least one rule failed.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "validation failed");
        }

        write!(f, "validation failed: ")?;
        let mut first = true;
        for error in &self.0 {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl From<ValidationError> for ValidationErrors {
    fn from(error: ValidationError) -> Self {
        Self(vec![error])
    }
}

impl From<Vec<ValidationError>> for ValidationErrors {
    fn from(errors: Vec<ValidationError>) -> Self {
        Self(errors)
    }
}

impl Extend<ValidationError> for ValidationErrors {
    fn extend<T: IntoIterator<Item = ValidationError>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl FromIterator<ValidationError> for ValidationErrors {
    fn from_iter<T: IntoIterator<Item = ValidationError>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ValidationErrors {
    type Item = &'a ValidationError;
    type IntoIter = std::slice::Iter<'a, ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Builds a `422 Unprocessable Entity` response carrying the failures as JSON.
///
/// The body has the shape `{"errors": [{"field", "message"}, ...]}` so API
/// clients and `x-target.422` partial swaps can act on each field. HTML pages
/// usually re-render the form with the errors inline instead; set the same
/// status by returning `(StatusCode::UNPROCESSABLE_ENTITY, view)`.
impl IntoResponse for ValidationErrors {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let body = serde_json::to_vec(&serde_json::json!({ "errors": self.errors() }))?;
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            body,
        )
            .into_response(cx)
    }
}

/// Prefixes an inner failure with its parent path, keeping code and message.
fn join_error(prefix: &str, index: Option<usize>, error: &ValidationError) -> ValidationError {
    let mut field = String::from(prefix);
    if let Some(index) = index {
        field.push('[');
        field.push_str(&index.to_string());
        field.push(']');
    }
    if !error.field().is_empty() {
        field.push('.');
        field.push_str(error.field());
    }

    let mut joined = ValidationError::new(field, error.message());
    if let Some(code) = error.code() {
        joined = joined.with_code(code);
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_bytes;

    struct Address {
        zip: String,
    }

    impl Validate for Address {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.check_with_code(
                "zip",
                !self.zip.trim().is_empty(),
                "required",
                "zip cannot be empty",
            );
            errors.into_result()
        }
    }

    struct Shipment {
        address: Address,
    }

    impl Validate for Shipment {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.nest("address", &self.address);
            errors.into_result()
        }
    }

    #[test]
    fn new_error_stores_field_and_message() {
        let error = ValidationError::new("email", "email is required");

        assert_eq!(error.field(), "email");
        assert_eq!(error.message(), "email is required");
        assert_eq!(error.to_string(), "email: email is required");
    }

    #[test]
    fn add_and_check_collect_in_order() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);

        errors.add("name", "name is required");
        errors.check("email", false, "email is required");
        errors.check("email", true, "this message is skipped");

        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors.errors(),
            &[
                ValidationError::new("name", "name is required"),
                ValidationError::new("email", "email is required"),
            ]
        );
    }

    #[test]
    fn field_views_filter_by_name() {
        let mut errors = ValidationErrors::new();
        errors.add("email", "first");
        errors.add("name", "other");
        errors.add("email", "second");

        assert!(errors.has_field("email"));
        assert!(!errors.has_field("missing"));
        assert_eq!(errors.field("email").count(), 2);
        assert_eq!(
            errors.messages("email").collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(errors.first_message("email"), Some("first"));
        assert_eq!(errors.first_message("missing"), None);
    }

    #[test]
    fn code_defaults_to_none_and_serializes_away() {
        let error = ValidationError::new("email", "email is required");

        assert_eq!(error.code(), None);

        let json = serde_json::to_value(&error).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({ "field": "email", "message": "email is required" })
        );
    }

    #[test]
    fn with_code_attaches_a_stable_tag() {
        let error = ValidationError::new("email", "email is required").with_code("required");

        assert_eq!(error.code(), Some("required"));
        assert_eq!(error.to_string(), "email: email is required");

        let json = serde_json::to_value(&error).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "field": "email",
                "message": "email is required",
                "code": "required",
            })
        );
    }

    #[test]
    fn coded_pushes_attach_codes() {
        let mut errors = ValidationErrors::new();
        errors.add_with_code("name", "required", "name cannot be empty");
        errors.check_with_code("email", false, "required", "email is required");
        errors.check_with_code("email", true, "required", "this message is skipped");

        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|error| error.code() == Some("required")));
    }

    #[test]
    fn nest_prefixes_inner_paths_and_keeps_codes() {
        let mut errors = ValidationErrors::new();
        errors.add("name", "name cannot be empty");
        errors.nest("address", &Address { zip: String::new() });

        assert!(errors.has_field("name"));
        assert!(errors.has_field("address.zip"));
        assert_eq!(
            errors.first_message("address.zip"),
            Some("zip cannot be empty")
        );
        assert_eq!(
            errors
                .field("address.zip")
                .next()
                .and_then(|error| error.code()),
            Some("required")
        );
    }

    #[test]
    fn nest_appends_nothing_when_inner_is_valid() {
        let mut errors = ValidationErrors::new();
        errors.nest(
            "address",
            &Address {
                zip: "12345".to_owned(),
            },
        );

        assert!(errors.is_empty());
    }

    #[test]
    fn nest_each_indexes_each_item() {
        let mut errors = ValidationErrors::new();
        errors.nest_each(
            "items",
            &[
                Address { zip: String::new() },
                Address {
                    zip: "12345".to_owned(),
                },
            ],
        );

        assert_eq!(errors.len(), 1);
        assert!(errors.has_field("items[0].zip"));
        assert!(!errors.has_field("items[1].zip"));
    }

    #[test]
    fn nesting_composes_through_levels() {
        let mut errors = ValidationErrors::new();
        errors.nest(
            "shipment",
            &Shipment {
                address: Address { zip: String::new() },
            },
        );
        errors.nest_each(
            "shipments",
            &[Shipment {
                address: Address { zip: String::new() },
            }],
        );

        assert!(errors.has_field("shipment.address.zip"));
        assert!(errors.has_field("shipments[0].address.zip"));
    }

    #[test]
    fn into_result_is_ok_only_when_empty() {
        assert!(ValidationErrors::new().into_result().is_ok());

        let mut errors = ValidationErrors::new();
        errors.add("title", "title cannot be empty");
        let result = errors.into_result();
        assert!(result.is_err());
    }

    #[test]
    fn display_joins_failures() {
        assert_eq!(ValidationErrors::new().to_string(), "validation failed");

        let mut errors = ValidationErrors::new();
        errors.add("title", "title cannot be empty");
        errors.add("email", "email is required");
        assert_eq!(
            errors.to_string(),
            "validation failed: title: title cannot be empty, email: email is required"
        );
    }

    #[test]
    fn from_impls_collect_single_and_many() {
        let single = ValidationErrors::from(ValidationError::new("a", "b"));
        assert_eq!(single.len(), 1);

        let many = ValidationErrors::from(vec![ValidationError::new("a", "b")]);
        assert_eq!(many.len(), 1);

        let collected: ValidationErrors = vec![
            ValidationError::new("a", "b"),
            ValidationError::new("c", "d"),
        ]
        .into_iter()
        .collect();
        assert_eq!(collected.len(), 2);
    }

    #[tokio::test]
    async fn into_response_renders_422_json() {
        let mut errors = ValidationErrors::new();
        errors.add("email", "email is required");
        errors.add_with_code("name", "required", "name cannot be empty");

        let response = errors
            .into_response(&Cx::default())
            .expect("response builds");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"application/json".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("body is JSON");
        assert_eq!(
            json,
            serde_json::json!({ "errors": [
                { "field": "email", "message": "email is required" },
                { "field": "name", "message": "name cannot be empty", "code": "required" },
            ] })
        );
    }
}
