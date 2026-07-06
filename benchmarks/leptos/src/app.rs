use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path, SsrMode,
};

use crate::{
    components::{SiteFooter, SiteNav},
    pages::{HomePage, ProductDetailPage, ProductsPage},
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="flex min-h-screen flex-col bg-slate-50 text-slate-900">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/storefront-leptos.css"/>
        <Title text="Meridian Supply"/>
        <Router>
            <SiteNav/>
            <main class="mx-auto w-full max-w-6xl flex-1 px-4 py-8">
                <Routes fallback=|| "Not found.">
                    <Route path=path!("") view=HomePage ssr=SsrMode::Async/>
                    <Route path=path!("products") view=ProductsPage ssr=SsrMode::Async/>
                    <Route path=path!("products/:id") view=ProductDetailPage ssr=SsrMode::Async/>
                </Routes>
            </main>
            <SiteFooter/>
        </Router>
    }
}
