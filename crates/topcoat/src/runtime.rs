#![doc = include_str!("../docs/runtime.md")]

pub use topcoat_runtime::*;
pub use topcoat_runtime_macro::*;

/// Renders the `<script>` tag that loads the browser runtime.
///
/// Place it in the `<head>` of every page that uses signals, event handlers,
/// bind attributes, procedures, or shards.
///
/// # Panics
///
/// Panics when the router was built without
/// [`runtime()`](RouterBuilderRuntimeExt::runtime), since the script would
/// have no routes to talk to.
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
