pub mod app;
#[cfg(feature = "ssr")]
pub mod catalog;
pub mod components;
pub mod format;
pub mod model;
pub mod pages;
pub mod server_fns;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
