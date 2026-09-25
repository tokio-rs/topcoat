#![doc = include_str!("../docs/runtime.md")]

pub use topcoat_runtime::*;
pub use topcoat_runtime_macro::*;

/// The `<script>` tag loading the browser runtime.
///
/// # Panics
///
/// Panics if the router was built without
/// [`runtime()`](RouterBuilderRuntimeExt::runtime).
#[cfg(feature = "view")]
#[topcoat::view::component]
pub async fn script(cx: &topcoat::context::Cx) -> topcoat::Result<impl topcoat::view::View> {
    assert!(
        topcoat::context::try_app_context::<RuntimeSetup>(cx).is_some(),
        "the browser runtime is not set up on this router; call `.runtime()` on the router builder",
    );
    Ok(topcoat::view::view! {
        <script type="module" src=(topcoat::runtime::SCRIPT)></script>
    })
}
