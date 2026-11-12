use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::html;

use crate::{cart::CartCount, data::ListingError, views::layout};

/// An HTML error page in the shared layout.
pub struct ErrorPage {
    status: StatusCode,
    cart: CartCount,
}

impl ErrorPage {
    pub fn not_found(cart: CartCount) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            cart,
        }
    }

    pub fn invalid(cart: CartCount) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            cart,
        }
    }

    pub fn from_listing(error: ListingError, cart: CartCount) -> Self {
        match error {
            ListingError::CategoryNotFound | ListingError::PageNotFound => Self::not_found(cart),
            ListingError::InvalidSort | ListingError::InvalidPage => Self::invalid(cart),
        }
    }
}

impl IntoResponse for ErrorPage {
    fn into_response(self) -> Response {
        let (title, message) = if self.status == StatusCode::NOT_FOUND {
            (
                "Page not found",
                "Sorry, we couldn't find the page you're looking for.",
            )
        } else {
            (
                "Invalid request",
                "Some of the filters in this link aren't valid. Try browsing from the catalog instead.",
            )
        };
        let content = html! {
            div class="mx-auto max-w-2xl px-4 py-24 text-center sm:py-32" {
                p class="text-sm font-semibold text-teal-700" { (self.status.as_u16()) }
                h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-900" { (title) }
                p class="mt-4 text-stone-600" { (message) }
                div class="mt-10 flex flex-wrap justify-center gap-4" {
                    a href="/" class="rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800" { "Back to home" }
                    a href="/products" class="rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100" { "Browse products" }
                }
            }
        };
        (self.status, layout(Some(title), self.cart, content)).into_response()
    }
}

/// Fallback for unknown paths.
pub async fn not_found_page(cart: CartCount) -> ErrorPage {
    ErrorPage::not_found(cart)
}
