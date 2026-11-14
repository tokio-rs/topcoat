use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::{
    components::{Footer, Header},
    model::PageError,
    pages::{AboutPage, DetailPage, ErrorPage, HomePage, ListingPage, OrdersPage},
};

/// The HTML document around the app.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="data:," />
                <HydrationScripts options islands=true />
                <MetaTags />
            </head>
            <body class="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased">
                <App />
            </body>
        </html>
    }
}

/// The storefront: the shared layout around the routed pages.
///
/// Everything here renders on the server only, except the islands.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/harbor-leptos-islands.css" />
        <Router>
            <a
                href="#main"
                class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow"
            >
                "Skip to content"
            </a>
            <Header />
            <main id="main" class="flex-1">
                <Routes fallback=|| view! { <ErrorPage error=PageError::NotFound /> }>
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/about") view=AboutPage />
                    <Route path=path!("/products") view=ListingPage />
                    <Route path=path!("/products/:id") view=DetailPage />
                    <Route path=path!("/admin/orders") view=OrdersPage />
                </Routes>
            </main>
            <Footer />
        </Router>
    }
}
