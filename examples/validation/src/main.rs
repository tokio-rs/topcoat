use std::{collections::HashSet, sync::Mutex};

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Router, RouterBuilderDiscoverExt, Slot, StatusCode,
        content::{Form, Json},
        href, layout, page, query_params, route,
    },
    validation::{Validate, ValidateWithCx, Validated, ValidationErrors, rules},
    view::{View, component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .discover()
            .app_context(UserDb::default())
            .build(),
    )
    .await
    .unwrap();
}

/// In-memory user table standing in for a database.
#[derive(Debug, Default)]
struct UserDb {
    names: Mutex<HashSet<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct Signup {
    name: String,
    email: String,
}

impl Validate for Signup {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check_with_code(
            "name",
            rules::required(&self.name),
            rules::CODE_REQUIRED,
            "name cannot be empty",
        );
        errors.check_with_code(
            "name",
            rules::max_length(&self.name, 40),
            rules::CODE_TOO_LONG,
            "name must be at most 40 characters",
        );
        if rules::required(&self.email) {
            errors.check_with_code(
                "email",
                rules::email(&self.email),
                rules::CODE_INVALID_EMAIL,
                "email must look like ada@example.com",
            );
        } else {
            errors.add_with_code("email", rules::CODE_REQUIRED, "email is required");
        }
        errors.into_result()
    }
}

#[layout("/")]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Validation"</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[component]
async fn signup_form(input: &Signup, errors: &ValidationErrors) -> Result<impl View> {
    Ok(view! {
        <form method="post" action=(href!(signup))>
            <label>
                "Name"
                <input type="text" name="name" value=(input.name.clone())>
            </label>
            if errors.has_field("name") {
                <p class="error">(errors.first_message("name").unwrap_or_default())</p>
            }
            <label>
                "Email"
                <input type="email" name="email" value=(input.email.clone())>
            </label>
            if errors.has_field("email") {
                <p class="error">(errors.first_message("email").unwrap_or_default())</p>
            }
            <button type="submit">"Sign up"</button>
        </form>
    })
}

#[page("/")]
async fn home() -> Result<impl View> {
    let input = Signup::default();
    let errors = ValidationErrors::new();
    Ok(view! {
        <h1>"Sign up"</h1>
        signup_form(input: &input, errors: &errors)
        <p>
            "The same rules guard "
            <code>"POST /api/users"</code>
            " as JSON, and "
            <a href=(href!(search))>"/search"</a>
            " shows query validation."
        </p>
    })
}

#[page(POST "/signup")]
async fn signup(Form(input): Form<Signup>) -> Result<impl View> {
    let result = input.validate();
    Ok(view! {
        match result {
            Ok(()) => {
                <h1>
                    "Welcome, "
                    (input.name.clone())
                    "!"
                </h1>
                <p>"Your account was created."</p>
            }
            Err(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY)
                <h1>"Sign up"</h1>
                signup_form(input: &input, errors: &errors)
            }
        }
    })
}

#[query_params(error = bad_request)]
struct Search {
    query: Option<String>,
    page: Option<u32>,
}

impl Validate for Search {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        if let Some(query) = &self.query {
            errors.check_with_code(
                "query",
                rules::max_length(query, 80),
                rules::CODE_TOO_LONG,
                "query must be at most 80 characters",
            );
        }
        if let Some(page) = self.page {
            errors.check_with_code(
                "page",
                rules::range(page, 1, 1000),
                rules::CODE_OUT_OF_RANGE,
                "page must be between 1 and 1000",
            );
        }
        errors.into_result()
    }
}

#[page("/search")]
async fn search(cx: &Cx) -> Result<impl View> {
    let params = query_params::<Search>(cx)?;
    params.validate()?;
    let query = params.query.clone().unwrap_or_default();
    Ok(view! {
        <h1>"Search"</h1>
        <form method="get" action=(href!(search))>
            <input type="text" name="query" value=(query.clone())>
            <button type="submit">"Search"</button>
        </form>
        if params.page.is_some() {
            <p>
                "Page "
                (params.page.unwrap_or(1).to_string())
            </p>
        }
    })
}

#[derive(Debug, Deserialize)]
struct Address {
    zip: String,
}

impl Validate for Address {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check_with_code(
            "zip",
            rules::required(&self.zip),
            rules::CODE_REQUIRED,
            "zip cannot be empty",
        );
        errors.check_with_code(
            "zip",
            rules::min_length(&self.zip, 5),
            rules::CODE_TOO_SHORT,
            "zip must be at least 5 characters",
        );
        errors.check_with_code(
            "zip",
            rules::max_length(&self.zip, 10),
            rules::CODE_TOO_LONG,
            "zip must be at most 10 characters",
        );
        errors.into_result()
    }
}

#[derive(Debug, Deserialize)]
struct NewUser {
    name: String,
    email: String,
    address: Address,
}

impl Validate for NewUser {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check_with_code(
            "name",
            rules::required(&self.name),
            rules::CODE_REQUIRED,
            "name cannot be empty",
        );
        errors.check_with_code(
            "email",
            rules::email(&self.email),
            rules::CODE_INVALID_EMAIL,
            "email must look like ada@example.com",
        );
        errors.nest("address", &self.address);
        errors.into_result()
    }
}

impl ValidateWithCx for NewUser {
    #[allow(clippy::unused_async_trait_impl)]
    async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        let taken = app_context::<UserDb>(cx)
            .names
            .lock()
            .expect("user table lock")
            .contains(&self.name);
        errors.check_with_code("name", !taken, "taken", "name is already taken");
        errors.into_result()
    }
}

#[route(POST "/api/users")]
async fn create_user(cx: &Cx, input: Validated<Json<NewUser>>) -> Result<Json<serde_json::Value>> {
    input.validate_cx(cx).await?;
    app_context::<UserDb>(cx)
        .names
        .lock()
        .expect("user table lock")
        .insert(input.name.clone());
    Ok(Json(serde_json::json!({ "name": input.name })))
}
