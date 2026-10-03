//! The runtime script component checks that the router is set up for the
//! runtime, so a page including the script on a router without the
//! runtime's routes fails at render time rather than in the browser.

use topcoat::{
    asset::{AssetConfig, Manifest, ManifestEntry},
    context::{Cx, CxTestBuilder},
    runtime::{RuntimeSetup, SCRIPT},
    view::{ViewExt, view},
};

/// An asset config resolving the runtime script, so the tag can render
/// without a bundle on disk.
fn asset_config() -> AssetConfig {
    let manifest = Manifest {
        version: 1,
        assets: vec![ManifestEntry {
            id: SCRIPT.id(),
            file: String::from("topcoat.js"),
            hash: String::new(),
            content_type: String::from("text/javascript"),
        }],
    };
    AssetConfig::hosted_at("/assets", manifest)
}

#[tokio::test]
async fn the_script_renders_on_a_router_set_up_for_the_runtime() {
    let cx = &CxTestBuilder::new()
        .app_context(asset_config())
        .app_context(RuntimeSetup)
        .build();
    let html = view! { cx => topcoat::runtime::script() }
        .single()
        .await
        .unwrap()
        .render(cx);
    assert!(html.contains(r#"src="/assets/topcoat.js""#), "{html}");
}

#[tokio::test]
#[should_panic(expected = "call `.runtime()` on the router builder")]
async fn the_script_panics_on_a_router_without_the_runtime() {
    let cx = &CxTestBuilder::new().app_context(asset_config()).build();
    let _ = view! { cx => topcoat::runtime::script() }.single().await;
}

#[tokio::test]
#[should_panic(expected = "call `.runtime()` on the router builder")]
async fn the_check_runs_before_the_asset_is_resolved() {
    let cx = &Cx::default();
    let _ = view! { cx => topcoat::runtime::script() }.single().await;
}
