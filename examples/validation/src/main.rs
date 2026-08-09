use serde::Deserialize;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, content::Form, page, route},
    validation::{
        FormSchema, ValidForm, ValidationEnv, ValidationError, ValidationErrors, Value,
        form_validation_handlers,
    },
    view::{component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .discover()
            .assets(AssetBundle::load().unwrap())
            .build(),
    )
    .await
    .unwrap();
}

/// A sign-up form with several different validators.
#[derive(Default, Deserialize, ValidForm)]
struct SignUp {
    #[validate(string, required, email)]
    email: String,
    #[validate(number, required, min = 18.0)]
    age: f64,
    #[validate(string, required, min_length = 3, max_length = 20)]
    username: String,
    #[validate(string, required, one_of = "user,admin")]
    role: String,
    #[validate(bool, or_default = false)]
    #[serde(default)]
    newsletter: bool,
    #[validate(string, server_only, custom = validate_referral)]
    referral: Option<String>,
}

fn validate_referral(value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
    match value {
        Value::Missing => Ok(Value::Missing),
        Value::String(text) if text.len() == 6 => Ok(value.clone()),
        _ => Err("referral code must be 6 characters".into()),
    }
}

/// Renders the sign-up form, optionally showing validation errors.
#[component]
async fn signup_form(
    #[default] errors: Option<&ValidationErrors>,
    signup: &SignUp,
) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Validation example"</title>
                topcoat::runtime::script()
            </head>
            <body>
                <h1>"Sign up"</h1>

                if let Some(errors) = errors {
                    <div style="color: red;">
                        <p>"Server-side errors:"</p>
                        <ul>
                            for (field, error) in errors.errors() {
                                <li>(field)": "(error.message())</li>
                            }
                        </ul>
                    </div>
                }

                signal client_errors = String::new();

                <div style="color: orange;">
                    $(client_errors.get())
                </div>

                <form
                    method="post"
                    action="/signup-manual"
                    (form_validation_handlers!(SignUp, client_errors))
                >
                    <label for="email">"Email"</label>
                    <input type="text" id="email" name="email" value=(signup.email.as_str())>
                    <br>

                    <label for="age">"Age"</label>
                    <input type="text" id="age" name="age" value=(signup.age)>
                    <br>

                    <label for="username">"Username"</label>
                    <input type="text" id="username" name="username" value=(signup.username.as_str())>
                    <br>

                    <label for="role">"Role"</label>
                    <select id="role" name="role">
                        <option value="user" if signup.role == "user" { selected="" }>"User"</option>
                        <option value="admin" if signup.role == "admin" { selected="" }>"Admin"</option>
                    </select>
                    <br>

                    <label>
                        <input type="checkbox" name="newsletter" value="true" if signup.newsletter { checked="" }>
                        "Subscribe to newsletter"
                    </label>
                    <br>

                    <label for="referral">"Referral code (server-only check)"</label>
                    <input type="text" id="referral" name="referral" value=(signup.referral.as_deref().unwrap_or(""))>
                    <br>

                    <button type="submit">"Sign up (manual validation)"</button>
                </form>

                <p>"Or POST the same form to <code>/signup</code> to use the <code>ValidForm</code> extractor."</p>
            </body>
        </html>
    }
}

/// Shows the sign-up form.
#[page("/signup")]
async fn signup_page() -> Result {
    let empty = SignUp::default();
    view! {
        signup_form(
            errors: None,
            signup: &empty,
        )
    }
}

/// Handles the form with the automatic <code>ValidForm</code> extractor.
#[route(POST "/signup")]
async fn signup_auto(ValidForm(signup): ValidForm<SignUp>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Signed up"</title>
            </head>
            <body>
                <h1>"Welcome, "(signup.username)"!"</h1>
                <ul>
                    <li>"Email: "(signup.email)</li>
                    <li>"Age: "(signup.age)</li>
                    <li>"Role: "(signup.role)</li>
                    <li>"Newsletter: "(signup.newsletter)</li>
                    <li>
                        "Referral: "
                        (signup.referral.as_deref().unwrap_or("none"))
                    </li>
                </ul>
                <a href="/signup">"Back"</a>
            </body>
        </html>
    }
}

/// Handles the form by validating it manually so errors can be rendered inline.
#[route(POST "/signup-manual")]
async fn signup_manual(Form(signup): Form<SignUp>) -> Result {
    match signup.validate_server() {
        Ok(_) => {
            view! {
                <!DOCTYPE html>
                <html>
                    <head>
                        <title>"Signed up"</title>
                    </head>
                    <body>
                        <h1>"Welcome, "(signup.username)"!"</h1>
                        <p>"Manual validation succeeded."</p>
                        <a href="/signup">"Back"</a>
                    </body>
                </html>
            }
        }
        Err(errors) => {
            view! {
                signup_form(
                    errors: Some(&errors),
                    signup: &signup,
                )
            }
        }
    }
}
