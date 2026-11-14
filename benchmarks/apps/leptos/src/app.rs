use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, provide_meta_context};
use leptos_router::{
    SsrMode,
    components::{Route, Router, Routes},
    path,
};

use crate::{
    cart::CartCount,
    components::{Footer, Header},
    model::PageError,
    pages::{AboutPage, DetailPage, ErrorPage, HomePage, ListingPage, OrdersPage},
};

/// The HTML document around the app. Rendered on the server only.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="data:," />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body class="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased">
                <App />
            </body>
        </html>
    }
}

/// The storefront: the shared layout around the routed pages.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    // Read from the request cookie on the server and sent along for hydration.
    let cart = SharedValue::new(initial_cart_count).into_inner();
    provide_context(RwSignal::new(cart));

    view! {
        <Stylesheet id="leptos" href="/pkg/harbor-leptos.css" />
        <Router>
            <a
                href="#main"
                class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow"
            >
                "Skip to content"
            </a>
            <Header />
            <main id="main" class="flex-1">
                // Pages with data render once it has loaded, so the title and the status code can
                // depend on it and the HTML is complete without JavaScript.
                <Routes fallback=|| view! { <ErrorPage error=PageError::NotFound /> }>
                    <Route path=path!("/") view=HomePage ssr=SsrMode::Async />
                    <Route path=path!("/about") view=AboutPage />
                    <Route path=path!("/products") view=ListingPage ssr=SsrMode::Async />
                    <Route path=path!("/products/:id") view=DetailPage ssr=SsrMode::Async />
                    <Route path=path!("/admin/orders") view=OrdersPage ssr=SsrMode::Async />
                </Routes>
            </main>
            <Footer />
        </Router>
    }
}

#[cfg(feature = "ssr")]
fn initial_cart_count() -> CartCount {
    CartCount::from_request()
}

#[cfg(not(feature = "ssr"))]
fn initial_cart_count() -> CartCount {
    CartCount::from_document()
}
