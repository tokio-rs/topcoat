//! Compiles the fixture consumed by the native browser suite.

use topcoat::{
    Result,
    context::Cx,
    runtime::{Expr, expr, signal},
    view::{View, ViewExt, component, view},
};

#[component]
async fn fixture(cx: &Cx) -> Result<impl View> {
    let state = signal(cx, || false);
    let alias = state.clone();
    let (read, read_js) = expr!(|| state.write_revision()).into_evaluated_and_js();
    assert!(
        std::panic::catch_unwind(|| {
            let _: Expr<u64> = expr!(state.write_revision());
        })
        .is_err()
    );
    assert!(std::panic::catch_unwind(read).is_err());
    let changed = expr!(|checkpoint| state.write_revision() != checkpoint);
    let alias_read = expr!(|| alias.write_revision());
    let capture = expr!(|| alias);

    Ok(view! {
        <div
            id="checkpoint"
            data-read=(read_js)
            data-changed=(changed.into_evaluated_and_js().1)
            data-alias-read=(alias_read.into_evaluated_and_js().1)
            data-capture=(capture.into_evaluated_and_js().1)
        >
            <button id="same" @click=$(|_e| state.set(false))>"Write false"</button>
            <button
                id="round-trip"
                @click=$(|_e| {
                    state.set(true);
                    state.set(false);
                })
            >
                "Write true then false"
            </button>
            <output id="value">$(state.get())</output>
            <form><input name="date" value="2026-01-31"></form>
        </div>
    })
}

#[tokio::test]
async fn compiled_browser_fixture() {
    let cx = &Cx::default();
    let html = view! { cx => fixture() }.single().await.unwrap().render(cx);
    assert!(!html.contains("::topcoat::dep("), "{html}");
    // The browser setup requests a fresh fixture from this exact checkout.
    if let Some(path) = std::env::var_os("TOPCOAT_REVISION_FIXTURE") {
        std::fs::write(path, html).unwrap();
    }
}
