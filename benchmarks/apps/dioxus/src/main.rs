//! Harbor Market storefront built with Dioxus fullstack: pages are rendered on the server with
//! Axum and hydrated in the browser by the same components compiled to WebAssembly.

mod cart;
mod format;
mod model;
mod pages;
mod routes;
mod views;

#[cfg(feature = "server")]
mod api;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
mod store;

use dioxus::prelude::*;

use crate::routes::Route;

#[cfg(all(feature = "jemalloc", feature = "mimalloc"))]
compile_error!("the `jemalloc` and `mimalloc` features are mutually exclusive");

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The Tailwind stylesheet that `dx` compiles from `tailwind.css`.
const STYLESHEET: Asset = asset!("/assets/tailwind.css");

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: STYLESHEET }
        Router::<Route> {}
    }
}

#[cfg(feature = "server")]
fn main() -> std::process::ExitCode {
    server::main()
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
