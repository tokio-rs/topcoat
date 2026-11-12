//! Harbor Market storefront built with Leptos in islands mode: server-side rendering with only
//! the interactive islands hydrated.

pub mod app;
mod cart;
mod components;
mod format;
mod model;
mod pages;
pub mod store;
mod urls;

#[cfg(feature = "ssr")]
pub mod api;

/// Entry point of the WebAssembly bundle: hydrates the islands on the server-rendered page.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
