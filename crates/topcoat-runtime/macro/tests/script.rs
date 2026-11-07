//! The runtime script component checks that the router is set up for the
//! runtime, so a page including the script on a router without the
//! runtime's routes fails at render time rather than in the browser.

use topcoat::{
    Result,
    asset::{AssetConfig, Manifest, ManifestEntry},
    context::Cx,
    router::{Body, Router, page, to_bytes},
    runtime::{RuntimeLayer, SCRIPT},
    view::{View, ViewExt, view},
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

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! { topcoat::runtime::script() })
}

#[tokio::test]
async fn the_script_renders_behind_the_runtime_layer() {
    let router = Router::builder()
        .page(home)
        .with(asset_config())
        .layer(RuntimeLayer::new())
        .build();
    let request = http::Request::builder()
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = router.handle(request).await;

    assert!(response.status().is_success(), "{}", response.status());
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains(r#"src="/assets/topcoat.js""#), "{html}");
}

#[tokio::test]
#[should_panic(expected = "add `.layer(RuntimeLayer::new())` to the router builder")]
async fn the_script_panics_on_a_router_without_the_runtime() {
    let cx = &Cx::new().with(asset_config());
    let _ = view! { topcoat::runtime::script() }.single().await;
}

#[tokio::test]
#[should_panic(expected = "add `.layer(RuntimeLayer::new())` to the router builder")]
async fn the_check_runs_before_the_asset_is_resolved() {
    let cx = &Cx::new();
    let _ = view! { topcoat::runtime::script() }.single().await;
}
