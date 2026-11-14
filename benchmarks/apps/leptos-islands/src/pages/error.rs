use leptos::prelude::*;

use crate::{components::PageTitle, model::PageError};

/// The page shown when a page cannot be rendered, with the matching HTTP status.
#[component]
pub fn ErrorPage(error: PageError) -> impl IntoView {
    let (status, title, message) = match error {
        PageError::NotFound => (
            404,
            "Page not found",
            "Sorry, we couldn't find the page you're looking for.",
        ),
        PageError::Invalid => (
            422,
            "Invalid request",
            "Some of the filters in this link aren't valid. Try browsing from the catalog instead.",
        ),
    };
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::from_u16(status).unwrap());
    }
    view! {
        <PageTitle title=title />
        <div class="mx-auto max-w-2xl px-4 py-24 text-center sm:py-32">
            <p class="text-sm font-semibold text-teal-700">{status}</p>
            <h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-900">{title}</h1>
            <p class="mt-4 text-stone-600">{message}</p>
            <div class="mt-10 flex flex-wrap justify-center gap-4">
                <a
                    href="/"
                    class="rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800"
                >
                    "Back to home"
                </a>
                <a
                    href="/products"
                    class="rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100"
                >
                    "Browse products"
                </a>
            </div>
        </div>
    }
}
