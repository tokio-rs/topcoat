Validate request input with ordinary functions and return every failure at once.

Parsing and validation are separate steps. Extractors like [`Form`](crate::router::content::Form) and [`Json`](crate::router::content::Json) reject a malformed body with `400 Bad Request`; validation covers a well formed value that breaks a domain rule, like an empty title or an unknown email shape. Implement [`Validate`] for the input type and call it from the handler. Rule failures respond `422 Unprocessable Entity` so clients can tell bad syntax apart from bad content.

# Basic usage

Implement [`Validate`] by collecting failures into [`ValidationErrors`] and finishing with `into_result`.

```rust
use topcoat::validation::{Validate, ValidationErrors, rules};

struct NewTodo {
    title: String,
}

impl Validate for NewTodo {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check("title", rules::required(&self.title), "title cannot be empty");
        errors.check(
            "title",
            rules::max_length(&self.title, 120),
            "title must be at most 120 characters",
        );
        errors.into_result()
    }
}
```

Call it from the handler with `?`. The collected errors convert into the handler error and the router renders them as JSON.

```rust,no_run
# use serde::Deserialize;
# use topcoat::{Result, router::{content::Form, route}, validation::{Validate, ValidationErrors}};
# #[derive(Deserialize)]
# struct NewTodo { title: String }
# impl Validate for NewTodo {
#     fn validate(&self) -> Result<(), ValidationErrors> {
#         ValidationErrors::new().into_result()
#     }
# }
#[route(POST "/todos")]
async fn create(Form(input): Form<NewTodo>) -> Result<&'static str> {
    input.validate()?;
    Ok("created")
}
```

A failing request responds `422` with the shape `{"errors": [{"field", "message"}, ...]}`, adding a `"code"` key to failures that carry one. Parse failures keep responding `400`, so API clients and `x-target.422` swaps can handle each case on its own.

# Parse and validate in one step

Take [`Validated`] as the body parameter to run the extractor and the [`Validate`] impl before the handler body starts. This rejects invalid input the same way as calling `validate` first, with less code. Only [`Validate`] runs here; if the type also implements [`ValidateWithCx`], call it in the handler.

```rust,no_run
# use serde::Deserialize;
# use topcoat::{Result, router::{content::Form, route}, validation::{Validate, Validated, ValidationErrors}};
# #[derive(Deserialize)]
# struct NewTodo { title: String }
# impl Validate for NewTodo {
#     fn validate(&self) -> Result<(), ValidationErrors> {
#         ValidationErrors::new().into_result()
#     }
# }
#[route(POST "/todos")]
async fn create(Validated(form): Validated<Form<NewTodo>>) -> Result<&'static str> {
    let _ = form;
    Ok("created")
}
```

Wrap it in [`Option`] to keep the body optional. [`Form`](crate::router::content::Form) and [`Json`](crate::router::content::Json) forward their [`Validate`] impls to the inner value, so `Validated<Form<T>>` only needs `T: Validate`.

# Error codes

Attach a stable machine-readable code next to the human message when API clients must act on a failure. Codes ride along into the `422` JSON body and are omitted when absent, so form-only handlers can ignore them. Prefer the canonical [`rules`] codes so clients match on the same strings across endpoints.

```rust
use topcoat::validation::{Validate, ValidationErrors, rules};

struct Signup {
    email: String,
}

impl Validate for Signup {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check_with_code(
            "email",
            rules::required(&self.email),
            rules::CODE_REQUIRED,
            "email is required",
        );
        errors.into_result()
    }
}
```

# Nested values

Validate a nested struct with `nest` and a sequence with `nest_each`. Inner paths join with dots (`"address"` plus `"zip"` becomes `"address.zip"`) and sequence items add their index (`"items[0].name"`), so nested failures stay addressable with the same field readers. Nesting composes: an inner value that nests itself keeps its full path, and codes survive the join.

```rust
use topcoat::validation::{Validate, ValidationErrors, rules};

struct Address {
    zip: String,
}

impl Validate for Address {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check("zip", rules::required(&self.zip), "zip cannot be empty");
        errors.into_result()
    }
}

struct Order {
    address: Address,
}

impl Validate for Order {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.nest("address", &self.address);
        errors.into_result()
    }
}
# let order = Order { address: Address { zip: String::new() } };
# let errors = order.validate().expect_err("a blank zip fails");
# assert_eq!(errors.first_message("address.zip"), Some("zip cannot be empty"));
```

Nested values arrive through JSON bodies, where objects nest. HTML forms post flat pairs, so form inputs stay flat structs.

# Query and path values

Query structs and path captures validate the same way. Parse them as usual, then call `validate` before using them.

```rust,no_run
# use topcoat::{Result, context::Cx, router::{page, query_params}, validation::{Validate, ValidationErrors}, view::{View, view}};
# #[query_params(error = bad_request)]
# struct Search { query: Option<String> }
# impl Validate for Search {
#     fn validate(&self) -> Result<(), ValidationErrors> {
#         ValidationErrors::new().into_result()
#     }
# }
#[page("/search")]
async fn search(cx: &Cx) -> Result<impl View> {
    let params = query_params::<Search>(cx)?;
    params.validate()?;
    Ok(view! { <h1>"Search"</h1> })
}
```

# Context checks

Pure rules stay in [`Validate`]. Checks that read request-scoped state, like a name that must be unique in the database, go in [`ValidateWithCx`], which receives `cx: &Cx`. Call it after [`Validate`] with `?` so sync failures answer before any query runs. Reach shared reads through `#[memoize]` helpers so repeated checks across a request run once.

```rust,no_run
# use topcoat::{Result, context::{Cx, app_context}, validation::{ValidateWithCx, ValidationErrors}};
# struct Database;
# impl Database {
#     async fn name_taken(&self, _name: &str) -> bool { false }
# }
# struct Signup { name: String }
impl ValidateWithCx for Signup {
    async fn validate_cx(&self, cx: &Cx) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();
        errors.check(
            "name",
            !app_context::<Database>(cx).name_taken(&self.name).await,
            "name is already taken",
        );
        errors.into_result()
    }
}
```

# Showing errors in a form

Answer the submission from a `POST` page and choose the markup inside one `view!` body, so both branches share a single view type. Failing input re-renders the form with status `422`, the submitted values, and the failures inline. Read one field with `first_message` and keep the typed values in the inputs so nothing the user wrote is lost.

```rust,no_run
# use serde::Deserialize;
# use topcoat::{Result, router::{StatusCode, content::Form, page}, validation::{Validate, ValidationErrors, rules}, view::{View, component, view}};
# #[derive(Deserialize, Default)]
# struct Signup { name: String, email: String }
# impl Validate for Signup {
#     fn validate(&self) -> Result<(), ValidationErrors> {
#         ValidationErrors::new().into_result()
#     }
# }
# #[component]
# async fn signup_form(input: &Signup, errors: &ValidationErrors) -> Result<impl View> {
#     let message = errors.first_message("email").unwrap_or_default();
#     Ok(view! { <p>(message)</p> })
# }
#[page(POST "/signup")]
async fn signup(Form(input): Form<Signup>) -> Result<impl View> {
    let result = input.validate();
    Ok(view! {
        match result {
            Ok(()) => {
                <h1>"Welcome, " (input.name.clone()) "!"</h1>
            },
            Err(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY)
                signup_form(input: &input, errors: &errors)
            },
        }
    })
}
```


# Rules

The [`rules`] module holds small predicates for the valid case: `required`, `length` with `min_length` and `max_length`, `range`, and `email`. Each rule documents its canonical code for `check_with_code`. Lengths count characters, not bytes. The email check is a fast heuristic for form input, not an RFC parse; use a dedicated address parser when deliverability must be proven.
