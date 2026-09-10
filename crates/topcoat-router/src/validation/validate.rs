use std::ops::{Deref, DerefMut};

use topcoat_core::{context::Cx, error::Result};

use super::error::ValidationErrors;
use crate::{
    Body,
    content::{Form, Json},
    request::{FromRequest, OptionalFromRequest},
};

/// Implemented by request input that can fail domain rules.
///
/// Parsing stays with the extractors: a malformed body is still `400 Bad
/// Request`. Implement this for the parsed type and check domain rules here.
/// Call it from the handler with `?`, or take [`Validated`] as the body
/// parameter to parse and validate in one step.
///
/// ```rust
/// use topcoat::validation::{Validate, ValidationErrors, rules};
///
/// struct NewTodo {
///     title: String,
/// }
///
/// impl Validate for NewTodo {
///     fn validate(&self) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         errors.check(
///             "title",
///             rules::required(&self.title),
///             "title cannot be empty",
///         );
///         errors.into_result()
///     }
/// }
/// ```
pub trait Validate {
    /// Checks the value against its domain rules.
    ///
    /// # Errors
    ///
    /// Returns the collected field failures when any rule fails.
    fn validate(&self) -> Result<(), ValidationErrors>;
}

impl<T> Validate for Form<T>
where
    T: Validate,
{
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.0.validate()
    }
}

impl<T> Validate for Json<T>
where
    T: Validate,
{
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.0.validate()
    }
}

/// Implemented by request input with rules that need the request context.
///
/// [`Validate`] covers pure rules. Implement this for checks that read
/// request-scoped state through `cx`: a name that must be unique in the
/// database, a coupon scoped to the current tenant, an email matching the
/// session. Reach shared reads through `#[memoize]` helpers so repeated
/// checks across a request run once.
///
/// Call it after [`Validate`] with `?`: sync failures answer before any
/// query runs.
///
/// Impls without an `.await` (a pure context read) need
/// `#[allow(clippy::unused_async_trait_impl)]`, like sync
/// [`FromRequest`](crate::request::FromRequest) impls.
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::{Cx, app_context},
///     validation::{ValidateWithCx, ValidationErrors},
/// };
///
/// # struct Database;
/// # impl Database {
/// #     async fn name_taken(&self, _name: &str) -> bool {
/// #         false
/// #     }
/// # }
/// struct Signup {
///     name: String,
/// }
///
/// impl ValidateWithCx for Signup {
///     async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         errors.check(
///             "name",
///             !app_context::<Database>(cx).name_taken(&self.name).await,
///             "name is already taken",
///         );
///         errors.into_result()
///     }
/// }
/// ```
pub trait ValidateWithCx {
    /// Checks the value against rules that read the request context.
    ///
    /// # Errors
    ///
    /// Returns the collected field failures when any rule fails.
    fn validate_cx(&self, cx: &Cx) -> impl Future<Output = Result<(), ValidationErrors>> + Send;
}

impl<T> ValidateWithCx for Form<T>
where
    T: ValidateWithCx + Sync,
{
    async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
        self.0.validate_cx(cx).await
    }
}

impl<T> ValidateWithCx for Json<T>
where
    T: ValidateWithCx + Sync,
{
    async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
        self.0.validate_cx(cx).await
    }
}

/// A body extractor that parses and then validates.
///
/// `Validated<Form<Signup>>` reads the form and checks its [`Validate`] impl
/// before the handler runs. A parse failure stays `400 Bad Request`; a rule
/// failure responds `422 Unprocessable Entity`. Wrap it in [`Option`] to keep
/// the body optional.
///
/// Only [`Validate`] runs here. If the type also implements [`ValidateWithCx`],
/// call `validate_cx` in the handler; context checks never run in the extractor.
///
/// ```rust
/// use serde::Deserialize;
/// use topcoat::{
///     Result,
///     router::{content::Form, route},
///     validation::{Validate, Validated, ValidationErrors},
/// };
///
/// #[derive(Deserialize)]
/// struct Signup {
///     email: String,
/// }
///
/// impl Validate for Signup {
///     fn validate(&self) -> Result<(), ValidationErrors> {
///         ValidationErrors::new().into_result()
///     }
/// }
///
/// #[route(POST "/signup")]
/// async fn signup(Validated(form): Validated<Form<Signup>>) -> Result<&'static str> {
///     let _ = form;
///     Ok("ok")
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Validated<T>(pub T);

impl<T> From<T> for Validated<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Validated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Validated<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Validate for Validated<T>
where
    T: Validate,
{
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.0.validate()
    }
}

impl<T> ValidateWithCx for Validated<T>
where
    T: ValidateWithCx + Sync,
{
    async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
        self.0.validate_cx(cx).await
    }
}

impl<T> FromRequest for Validated<T>
where
    T: FromRequest + Validate,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let inner = T::from_request(cx, body).await?;
        inner.validate()?;
        Ok(Self(inner))
    }
}

impl<T> OptionalFromRequest for Validated<T>
where
    T: OptionalFromRequest + Validate,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        let Some(inner) = <T as OptionalFromRequest>::from_request(cx, body).await? else {
            return Ok(None);
        };
        inner.validate()?;
        Ok(Some(Self(inner)))
    }
}

#[cfg(test)]
mod tests {
    use http::{Method, Request, header::CONTENT_TYPE};
    use serde::Deserialize;
    use topcoat_core::context::{CxTestBuilder, app_context};

    use super::*;
    use crate::{
        content::{Form, Json},
        validation::rules,
    };

    #[derive(Debug, Deserialize)]
    struct Signup {
        email: String,
    }

    impl Validate for Signup {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.check("email", rules::required(&self.email), "email is required");
            errors.check(
                "email",
                rules::email(&self.email),
                "email must look like ada@example.com",
            );
            errors.into_result()
        }
    }

    #[derive(Default)]
    struct FakeDb {
        taken: Vec<String>,
    }

    struct UniqueName {
        name: String,
    }

    impl ValidateWithCx for UniqueName {
        #[allow(clippy::unused_async_trait_impl)]
        async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            let taken = app_context::<FakeDb>(cx)
                .taken
                .iter()
                .any(|name| name == &self.name);
            errors.check_with_code("name", !taken, "taken", "name is already taken");
            errors.into_result()
        }
    }

    fn cx_with_db(db: FakeDb) -> Cx {
        CxTestBuilder::new().app_context(db).build()
    }

    struct Team {
        lead: UniqueName,
        members: Vec<UniqueName>,
    }

    impl ValidateWithCx for Team {
        async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.nest_cx("lead", &self.lead, cx).await;
            errors.nest_each_cx("members", &self.members, cx).await;
            errors.into_result()
        }
    }

    #[tokio::test]
    async fn nest_cx_prefixes_async_failures() {
        let cx = cx_with_db(FakeDb {
            taken: vec!["ada".to_owned()],
        });
        let team = Team {
            lead: UniqueName {
                name: "ada".to_owned(),
            },
            members: vec![
                UniqueName {
                    name: "grace".to_owned(),
                },
                UniqueName {
                    name: "ada".to_owned(),
                },
            ],
        };
        let errors = team.validate_cx(&cx).await.expect_err("taken names fail");

        assert!(errors.has_field("lead.name"));
        assert!(errors.has_field("members[1].name"));
        assert!(!errors.has_field("members[0].name"));
        assert_eq!(
            errors.field("lead.name").next().map(|error| error.code()),
            Some(Some("taken"))
        );
    }

    #[tokio::test]
    async fn nest_cx_appends_nothing_when_inner_is_valid() {
        let cx = cx_with_db(FakeDb::default());
        let team = Team {
            lead: UniqueName {
                name: "ada".to_owned(),
            },
            members: vec![],
        };
        team.validate_cx(&cx).await.expect("free names pass");
    }

    #[tokio::test]
    async fn validate_cx_rejects_taken_names() {
        let cx = cx_with_db(FakeDb {
            taken: vec!["ada".to_owned()],
        });
        let errors = UniqueName {
            name: "ada".to_owned(),
        }
        .validate_cx(&cx)
        .await
        .expect_err("a taken name fails");

        assert_eq!(errors.first_message("name"), Some("name is already taken"));
        assert_eq!(
            errors.field("name").next().map(|error| error.code()),
            Some(Some("taken"))
        );
    }

    #[tokio::test]
    async fn validate_cx_accepts_free_names() {
        let cx = cx_with_db(FakeDb::default());
        UniqueName {
            name: "ada".to_owned(),
        }
        .validate_cx(&cx)
        .await
        .expect("a free name passes");
    }

    #[tokio::test]
    async fn wrappers_delegate_validate_cx() {
        let cx = cx_with_db(FakeDb {
            taken: vec!["ada".to_owned()],
        });
        let error = Form(UniqueName {
            name: "ada".to_owned(),
        })
        .validate_cx(&cx)
        .await
        .expect_err("delegation fails like the inner value");
        assert!(error.has_field("name"));

        let cx = cx_with_db(FakeDb::default());
        Validated(Json(UniqueName {
            name: "ada".to_owned(),
        }))
        .validate_cx(&cx)
        .await
        .expect("delegation passes like the inner value");
    }

    fn cx(method: Method, uri: &str, content_type: Option<&str>) -> Cx {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(content_type) = content_type {
            builder = builder.header(CONTENT_TYPE, content_type);
        }

        let (parts, ()) = builder.body(()).expect("request should build").into_parts();

        CxTestBuilder::new().request_context(parts).build()
    }

    #[test]
    fn form_and_json_delegate_to_inner_value() {
        let valid = Signup {
            email: "ada@example.com".to_owned(),
        };
        assert!(Form(valid).validate().is_ok());

        let invalid = Signup {
            email: String::new(),
        };
        assert!(Json(invalid).validate().is_err());
    }

    #[test]
    fn validate_collects_rule_failures() {
        let input = Signup {
            email: "not-an-email".to_owned(),
        };

        let errors = input.validate().expect_err("a bad email fails");
        assert!(errors.has_field("email"));
    }

    #[tokio::test]
    async fn validated_form_accepts_valid_input() {
        let cx = cx(
            Method::POST,
            "/signup",
            Some("application/x-www-form-urlencoded"),
        );
        let Validated(form) = <Validated<Form<Signup>> as FromRequest>::from_request(
            &cx,
            Body::from("email=ada%40example.com"),
        )
        .await
        .expect("a valid form");

        assert_eq!(form.email, "ada@example.com");
    }

    #[tokio::test]
    async fn validated_form_rejects_rule_failures() {
        let cx = cx(
            Method::POST,
            "/signup",
            Some("application/x-www-form-urlencoded"),
        );
        let error =
            <Validated<Form<Signup>> as FromRequest>::from_request(&cx, Body::from("email="))
                .await
                .expect_err("an empty email fails validation");

        assert!(error.downcast_ref::<ValidationErrors>().is_some());
    }

    #[tokio::test]
    async fn validated_form_keeps_parse_errors_as_bad_request() {
        let cx = cx(Method::POST, "/signup", None);
        let error =
            <Validated<Form<Signup>> as FromRequest>::from_request(&cx, Body::from("email=a"))
                .await
                .expect_err("a missing content type is a parse error");

        assert!(
            error
                .downcast_ref::<crate::error::BadRequestError>()
                .is_some()
        );
    }

    #[tokio::test]
    async fn optional_validated_form_is_none_without_body() {
        let cx = cx(Method::POST, "/signup", None);
        let form = <Validated<Form<Signup>> as OptionalFromRequest>::from_request(
            &cx,
            Body::from("email=a"),
        )
        .await
        .expect("an absent body is not an error");

        assert!(form.is_none());
    }
}
