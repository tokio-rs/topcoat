//! Harbor Market storefront built with Leptos: server-side rendering with whole-app hydration.

pub mod app;
mod cart;
mod components;
mod format;
mod model;
mod pages;
mod urls;

#[cfg(feature = "ssr")]
pub mod api;
#[cfg(feature = "ssr")]
pub mod store;

/// Entry point of the WebAssembly bundle: hydrates the server-rendered page.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
