use dioxus::prelude::*;

use super::{Footer, Header};
use crate::{Route, cart::cart_count};

/// The page shell shared by every route: skip link, header, the routed page and the footer.
///
/// Loads the cart count from the request's cookie on the server and provides it to the page as a
/// [`CartCount`](crate::cart::CartCount) signal, which the add-to-cart button updates in the
/// browser.
#[component]
pub fn Shell() -> Element {
    let initial = use_server_future(cart_count)?;
    let initial = initial().and_then(Result::ok).unwrap_or_default();
    use_context_provider(|| Signal::new(initial));

    rsx! {
        a {
            href: "#content",
            class: "sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow",
            "Skip to content"
        }
        Header {}
        main { id: "content", class: "flex-1", Outlet::<Route> {} }
        Footer {}
    }
}
