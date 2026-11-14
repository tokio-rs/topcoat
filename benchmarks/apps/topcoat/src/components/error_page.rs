use std::fmt;

use topcoat::{
    Error, Result,
    context::Cx,
    router::{StatusCode, error::NotFoundError, href},
    view::{View, component, view},
};

use crate::app::{self, products};

/// Filters in a link that are not valid, answered with the 422 error page.
#[derive(Debug, Clone, Copy)]
pub struct InvalidRequestError;

impl fmt::Display for InvalidRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid request")
    }
}

impl std::error::Error for InvalidRequestError {}

/// The error pages a page can fail with.
#[derive(Clone, Copy)]
pub enum PageError {
    NotFound,
    InvalidRequest,
}

impl PageError {
    /// The page error for `error`, or `None` if the error is not one the error pages show.
    pub fn of(error: &Error) -> Option<Self> {
        if error.downcast_ref::<NotFoundError>().is_some() {
            Some(Self::NotFound)
        } else if error.downcast_ref::<InvalidRequestError>().is_some() {
            Some(Self::InvalidRequest)
        } else {
            None
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::NotFound => "Page not found",
            Self::InvalidRequest => "Invalid request",
        }
    }

    /// The document title of the error page.
    pub fn document_title(self) -> &'static str {
        match self {
            Self::NotFound => "Page not found | Harbor Market",
            Self::InvalidRequest => "Invalid request | Harbor Market",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidRequest => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotFound => "Sorry, we couldn't find the page you're looking for.",
            Self::InvalidRequest => {
                "Some of the filters in this link aren't valid. Try browsing from the catalog instead."
            }
        }
    }
}

/// The content of an error page. Also sets the response status.
#[component]
pub async fn error_page(cx: &Cx, error: PageError) -> Result<impl View> {
    let status = error.status();
    Ok(view! {
        (status)
        <div class="mx-auto max-w-2xl px-4 py-24 text-center sm:py-32">
            <p class="text-sm font-semibold text-teal-700">(status.as_u16())</p>
            <h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-900">
                (error.title())
            </h1>
            <p class="mt-4 text-stone-600">(error.message())</p>
            <div class="mt-10 flex flex-wrap justify-center gap-4">
                <a
                    href=(href!(app::home))
                    class="rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800"
                >
                    "Back to home"
                </a>
                <a
                    href=(href!(products::listing))
                    class="rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100"
                >
                    "Browse products"
                </a>
            </div>
        </div>
    })
}
