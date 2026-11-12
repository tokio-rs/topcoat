use dioxus::{fullstack::FullstackContext, prelude::*};

use crate::{model::Sort, routes::Route};

/// The error pages of the store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageError {
    NotFound,
    Invalid,
}

impl PageError {
    fn status(self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

/// An error page in the shared layout. On the server it also sets the response status.
#[component]
pub fn ErrorPage(error: PageError) -> Element {
    use_hook(|| FullstackContext::commit_http_status(error.status(), None));
    let (title, message) = match error {
        PageError::NotFound => (
            "Page not found",
            "Sorry, we couldn't find the page you're looking for.",
        ),
        PageError::Invalid => (
            "Invalid request",
            "Some of the filters in this link aren't valid. Try browsing from the catalog instead.",
        ),
    };
    let status = error.status().as_u16();
    rsx! {
        document::Title { "{title} | Harbor Market" }
        div { class: "mx-auto max-w-2xl px-4 py-24 text-center sm:py-32",
            p { class: "text-sm font-semibold text-teal-700", "{status}" }
            h1 { class: "mt-3 text-4xl font-semibold tracking-tight text-stone-900", {title} }
            p { class: "mt-4 text-stone-600", {message} }
            div { class: "mt-10 flex flex-wrap justify-center gap-4",
                Link {
                    to: Route::Home {},
                    class: "rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800",
                    "Back to home"
                }
                Link {
                    to: Route::listing(None, Sort::default(), 1),
                    class: "rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100",
                    "Browse products"
                }
            }
        }
    }
}

/// Renders the error page for a server function error with a 404 or 422 status, and passes any
/// other error up to the nearest error boundary.
pub fn error_page(error: &ServerFnError) -> Element {
    let error = match error {
        ServerFnError::ServerError { code: 404, .. } => PageError::NotFound,
        ServerFnError::ServerError { code: 422, .. } => PageError::Invalid,
        other => return Err(other.clone().into()),
    };
    rsx! { ErrorPage { error } }
}

/// Fallback route for unknown paths.
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    _ = segments;
    rsx! { ErrorPage { error: PageError::NotFound } }
}
