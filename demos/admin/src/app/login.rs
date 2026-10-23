use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
    icon::{icon, iconify::iconify_icon},
    l10n::{Translation, fluent::messages},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, redirect, rewrite, see_other},
        href, page, route,
    },
    view::{View, attributes, element_id, view},
};

use super::{_common::role_label, language::language_menu};
use crate::{
    components::{
        badge::{BadgeVariant, badge},
        button::button,
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        field::{field, field_error, field_group, field_label},
        input::input,
    },
    features::{
        auth::{current_user, sign_in},
        user::User,
    },
    seed::{DEMO_PASSWORD, DEMO_USERS},
};

messages!("{mod}/l10n/{locale}.ftl");

struct SignInFailed {
    email: String,
    email_error: Option<SignInError>,
    password_error: Option<SignInError>,
}

#[derive(Clone, Copy)]
enum SignInError {
    EmailMissing,
    PasswordMissing,
    WrongCredentials,
}

impl SignInError {
    fn message(self) -> Translation<'static> {
        match self {
            Self::EmailMissing => t!(error_email_missing),
            Self::PasswordMissing => t!(error_password_missing),
            Self::WrongCredentials => t!(error_wrong_credentials),
        }
    }
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    if current_user(cx).await?.is_some() {
        return Err(redirect(href!(super::_admin::_dashboard::page).resolve(cx)).into());
    }

    let failed = try_request_context::<SignInFailed>(cx);
    let email = failed.map(|failed| failed.email.as_str());
    let email_error = failed.and_then(|failed| failed.email_error);
    let password_error = failed.and_then(|failed| failed.password_error);

    let form_id = element_id(cx);
    let email_id = element_id(cx);
    let email_error_id = element_id(cx);
    let password_id = element_id(cx);
    let password_error_id = element_id(cx);

    Ok(view! {
        if failed.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <main
            class="relative flex min-h-svh flex-col items-center justify-center gap-6 p-6"
        >
            <div class="absolute top-3 right-3">language_menu()</div>

            <p class="flex items-center gap-2 text-lg font-semibold">
                icon(data: iconify_icon!("lucide:store"))
                "Backroom"
            </p>

            card(
                attrs: attributes! { class="w-full max-w-sm" },
                card_header(
                    card_title((t!(sign_in)))
                    card_description((t!(sign_in.description)))
                )
                card_content(
                    // Disable browser validation so the form shows the server's field errors.
                    <form
                        id=(&form_id)
                        method="post"
                        action=(href!(submit))
                        novalidate=""
                    >
                        field_group(
                            field(
                                field_label(
                                    attrs: attributes! { for=(&email_id) },
                                    (t!(email))
                                )
                                input(
                                    attrs: attributes! {
                                        id=(&email_id)
                                        name="email"
                                        type="email"
                                        autocomplete="username"
                                        value=(email)
                                        aria-invalid=(email_error.is_some().then_some("true"))
                                        aria-describedby=(email_error
                                            .is_some()
                                            .then_some(&email_error_id))
                                    }
                                )
                                if let Some(error) = email_error {
                                    field_error(
                                        attrs: attributes! { id=(&email_error_id) },
                                        (error.message())
                                    )
                                }
                            )
                            field(
                                field_label(
                                    attrs: attributes! { for=(&password_id) },
                                    (t!(password))
                                )
                                input(
                                    attrs: attributes! {
                                        id=(&password_id)
                                        name="password"
                                        type="password"
                                        autocomplete="current-password"
                                        aria-invalid=(password_error.is_some().then_some("true"))
                                        aria-describedby=(password_error
                                            .is_some()
                                            .then_some(&password_error_id))
                                    }
                                )
                                if let Some(error) = password_error {
                                    field_error(
                                        attrs: attributes! { id=(&password_error_id) },
                                        (error.message())
                                    )
                                }
                            )
                        )
                    </form>
                )
                card_footer(
                    button(
                        attrs: attributes! { type="submit" form=(&form_id) class="w-full" },
                        (t!(sign_in))
                    )
                )
            )

            demo_accounts()
        </main>
    })
}

#[topcoat::view::component]
async fn demo_accounts() -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="w-full max-w-sm" },
            card_header(
                card_title((t!(demo_accounts)))
                // A message cannot contain markup, so the password follows the sentence.
                card_description(
                    (t!(demo_accounts.description))
                    " "
                    <code class="font-mono text-foreground">(DEMO_PASSWORD)</code>
                )
            )
            card_content(
                <ul class="flex flex-col gap-3 text-sm">
                    for (name, email, role) in DEMO_USERS {
                        <li class="flex items-center justify-between gap-4">
                            <div class="flex flex-col">
                                <span class="font-medium">(name)</span>
                                <span class="text-muted-foreground">(email)</span>
                            </div>
                            badge(variant: BadgeVariant::Outline, (role_label(role)))
                        </li>
                    }
                </ul>
            )
        )
    })
}

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

#[route(POST)]
async fn submit(cx: &Cx, Form(credentials): Form<Credentials>) -> Result<SeeOther> {
    let Credentials { email, password } = credentials;
    let mut email_error = email.trim().is_empty().then_some(SignInError::EmailMissing);
    let password_error = password.is_empty().then_some(SignInError::PasswordMissing);

    if email_error.is_none() && password_error.is_none() {
        if let Some(user) = User::authenticate(cx, &email, password).await? {
            sign_in(cx, &user).await?;
            return Ok(see_other(
                href!(super::_admin::_dashboard::page).resolve(cx),
            ));
        }
        email_error = Some(SignInError::WrongCredentials);
    }

    // Rewrite the POST to keep the submitted email and errors in the request context.
    Err(rewrite(href!(page).resolve(cx), Body::empty())
        .method(Method::GET)
        .with(SignInFailed {
            email,
            email_error,
            password_error,
        })
        .into())
}
