//! The runtime script component checks that the router is set up for the
//! runtime, so a page including the script on a router without the
//! runtime's routes fails at render time rather than in the browser.

use topcoat::{
    context::{Cx, CxTestBuilder},
    runtime::RuntimeSetup,
    view::{ViewExt, view},
};

#[tokio::test]
async fn the_script_renders_on_a_router_set_up_for_the_runtime() {
    let cx = &CxTestBuilder::new().app_context(RuntimeSetup).build();
    let html = view! { cx => topcoat::runtime::script() }
        .single()
        .await
        .unwrap()
        .render(cx);
    assert!(html.contains("<script"), "{html}");
}

#[tokio::test]
#[should_panic(expected = "call `.runtime()` on the router builder")]
async fn the_script_panics_on_a_router_without_the_runtime() {
    let cx = &Cx::default();
    let _ = view! { cx => topcoat::runtime::script() }.single().await;
}
