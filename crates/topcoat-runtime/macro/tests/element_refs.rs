use std::panic::catch_unwind;

use topcoat::{
    Result,
    context::Cx,
    runtime::{DialogElement, ElementRef, HtmlElement, InputElement, expr},
    view::{View, ViewExt, component, element_id, view},
};

#[test]
fn captures_preserve_ids_and_expected_types() {
    let input = ElementRef::<InputElement>::new("tc-field");
    let borrowed = &input;
    let captured = expr!(borrowed.clone());
    assert_eq!(captured.into_evaluated_and_js().0.id(), input.id());
    assert_eq!(
        expr!(input.id().to_owned()).into_evaluated_and_js().0,
        input.id()
    );

    // These signatures fail to compile if resolving loses the expected type.
    let _input = expr!(|| -> InputElement { input.resolve().unwrap() });
    let dialog = ElementRef::<DialogElement>::new("tc-dialog");
    let _dialog = expr!(|| -> DialogElement { dialog.resolve().unwrap() });
    let html = ElementRef::<HtmlElement>::new("tc-container");
    let _html = expr!(|| -> HtmlElement { html.resolve().unwrap() });
}

#[test]
fn server_evaluation_cannot_resolve_elements() {
    let input = ElementRef::<InputElement>::new("tc-field");
    assert!(catch_unwind(|| expr!(input.resolve())).is_err());
    let (handler, _) = expr!(|| input.resolve().is_ok()).into_evaluated_and_js();
    assert!(catch_unwind(handler).is_err());
}

#[component]
async fn field(cx: &Cx, key: usize) -> Result<impl View> {
    let input = ElementRef::<InputElement>::new(element_id(cx));
    Ok(view! {
        <input id=(input.id()) data-key=(key)>
        <button
            data-key=(key)
            @click=$(|_e| {
                input.resolve().unwrap();
            })
        >
            "Resolve"
        </button>
    })
}

async fn keyed_fields(keys: &[usize]) -> String {
    let cx = &Cx::default();
    view! {
        cx =>
        #[key(key)]
        for key in keys {
            field(key: *key)
        }
    }
    .single()
    .await
    .unwrap()
    .render(cx)
}

#[tokio::test]
async fn browser_fixture_matches_compiled_expressions_and_keyed_markup() {
    let input = ElementRef::<InputElement>::new("tc-field");
    let dialog = ElementRef::<DialogElement>::new("tc-dialog");
    let html = ElementRef::<HtmlElement>::new("tc-container");
    let fixture = serde_json::json!({
        "input": expr!(|| input.resolve()).into_evaluated_and_js().1.to_source(),
        "dialog": expr!(|| dialog.resolve()).into_evaluated_and_js().1.to_source(),
        "html": expr!(|| html.resolve()).into_evaluated_and_js().1.to_source(),
        "id": expr!(input.id().to_owned()).into_evaluated_and_js().1.to_source(),
        "missing": expr!(|| input.resolve().unwrap_err().is_missing()).into_evaluated_and_js().1.to_source(),
        "wrong_type": expr!(|| input.resolve().unwrap_err().is_wrong_type()).into_evaluated_and_js().1.to_source(),
        "keyed": keyed_fields(&[1, 2]).await,
        "reordered": keyed_fields(&[2, 1]).await,
    });
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../browser/tests/element-refs.json"
    );
    if std::env::var_os("UPDATE_ELEMENT_REF_FIXTURE").is_some() {
        std::fs::write(path, serde_json::to_string_pretty(&fixture).unwrap() + "\n").unwrap();
    }
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        fixture, expected,
        "regenerate with UPDATE_ELEMENT_REF_FIXTURE=1 cargo test -p topcoat-runtime-macro --test element_refs"
    );
}
