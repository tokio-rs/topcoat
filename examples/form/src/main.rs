use std::collections::HashMap;

use serde::Deserialize;
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    form::{
        checkbox, form_group_handlers, form_validation_handlers, number_input, text_input,
        FormGroup, FormGroupSchema, FormSchema, ValidForm, ValidationEnv, ValidationError,
        ValidationErrors, Value,
    },
    router::{content::Form, page, route, Router, RouterBuilderDiscoverExt},
    view::{component, view},
    Result,
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
async fn signup_form(#[default] errors: Option<&ValidationErrors>, signup: &SignUp) -> Result {
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
        Ok(()) => {
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

/// A sign-up form validated live by the server through a [`FormGroup`].
///
/// `#[form_view]` names the component the generated update procedure re-renders
/// on every input. The derive also generates `SignUpGroup::form_update_procedure`.
#[derive(Default, Deserialize, ValidForm)]
#[form_view(signup_group_form)]
struct SignUpGroup {
    #[validate(string, required, email)]
    email: String,
    #[validate(number, required, min = 18.0)]
    age: f64,
    #[validate(string, required, min_length = 3, max_length = 20)]
    username: String,
    #[validate(bool, or_default = false)]
    #[serde(default)]
    newsletter: bool,
}

/// Renders only the `<form>` fragment for a [`FormGroup<SignUpGroup>`].
///
/// This is the component named by `#[form_view]`: the update procedure
/// re-renders it on every input and the client swaps it into the DOM, replacing
/// the existing form element (`outer` mode) so the `data-control-*` state and
/// error messages always reflect the server's view.
#[component]
async fn signup_group_form(group: &FormGroup<SignUpGroup>) -> Result {
    let email = SignUpGroup::email_control(group);
    let username = SignUpGroup::username_control(group);
    let age = SignUpGroup::age_control(group);
    let newsletter = SignUpGroup::newsletter_control(group);

    view! {
        <form
            method="post"
            action="/signup-group"
            (form_group_handlers!(SignUpGroup, "signup-group"))
        >
            text_input(
                control: email,
                name: "email",
                label: "Email",
                input_type: "email",
            )
            text_input(
                control: username,
                name: "username",
                label: "Username",
            )
            number_input(
                control: age,
                name: "age",
                label: "Age",
            )
            checkbox(
                control: newsletter,
                name: "newsletter",
                label: "Subscribe to newsletter",
            )

            <button type="submit">"Sign up"</button>
        </form>
    }
}

/// Renders the full page around the live-validated form fragment.
#[component]
async fn signup_group_page_view(group: &FormGroup<SignUpGroup>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"FormGroup validation"</title>
                topcoat::runtime::script()
            </head>
            <body>
                <h1>"Sign up (live FormGroup validation)"</h1>

                signup_group_form(group: group)

                <p><a href="/signup">"Back to the classic form"</a></p>
            </body>
        </html>
    }
}

/// Shows the live-validated form.
#[page("/signup-group")]
async fn signup_group_page() -> Result {
    let group = SignUpGroup::form_group();
    view! {
        signup_group_page_view(group: &group)
    }
}

/// Handles the final POST, assembling the validated struct from the group.
#[route(POST "/signup-group")]
async fn signup_group_submit(Form(values): Form<HashMap<String, String>>) -> Result {
    if let Ok(signup) = SignUpGroup::form_group_from_values(ValidationEnv::Server, &values)
        .and_then(|group| group.value())
    {
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
                        <li>"Newsletter: "(signup.newsletter)</li>
                    </ul>
                    <a href="/signup-group">"Back"</a>
                </body>
            </html>
        }
    } else {
        // Re-render with the submitted (invalid) values so errors show.
        let group = SignUpGroup::form_group_from_values_lossy(ValidationEnv::Server, &values);
        view! {
            signup_group_page_view(group: &group)
        }
    }
}

#[cfg(test)]
mod tests {
    use topcoat::form::{FormControl, FormGroupSchema};

    use super::SignUpGroup;

    #[test]
    fn generated_control_accessors_have_correct_types() {
        let group = SignUpGroup::form_group();
        let email: &FormControl<String> = SignUpGroup::email_control(&group);
        let age: &FormControl<f64> = SignUpGroup::age_control(&group);
        let newsletter: &FormControl<bool> = SignUpGroup::newsletter_control(&group);

        assert_eq!(email.value(), "");
        assert_eq!(*age.value(), 0.0);
        assert!(!*newsletter.value());
    }
}
