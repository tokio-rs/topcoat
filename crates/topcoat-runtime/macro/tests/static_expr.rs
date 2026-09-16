use topcoat::{
    Result,
    context::Cx,
    runtime::{Expr, expr, signal},
    view::{View, ViewExt, attributes, component, view},
};

#[component]
async fn panel(#[into] open: Expr<bool>) -> Result<impl View> {
    Ok(view! { <dialog :open=(open)></dialog> })
}

#[tokio::test]
async fn converted_props_render_as_ordinary_attributes() {
    let cx = &Cx::default();
    let html = view! {
        cx =>
        panel(open: true)
        panel(open: false)
    }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert_eq!(html, "<dialog open=\"\"></dialog><dialog></dialog>");
}

#[component]
async fn interactive_panel(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);
    Ok(view! { panel(open: $(open.get())) })
}

#[tokio::test]
async fn runtime_props_keep_their_bindings() {
    let cx = &Cx::default();
    let html = view! { cx => interactive_panel() }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("data-topcoat-bind:open="), "{html}");
    assert!(html.contains(".get()"), "{html}");
}

#[tokio::test]
async fn converted_nodes_render_without_markers_and_escape_html() {
    let cx = &Cx::default();
    let text = Expr::from("<b>&");
    let html = view! { cx => <p>(text)</p> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert_eq!(html, "<p>&lt;b&gt;&amp;</p>");
}

#[tokio::test]
async fn explicit_runtime_constants_keep_their_markers() {
    let cx = &Cx::default();
    assert!(!expr!(true).is_static());
    let html = view! { cx => <p>$("constant")</p> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("::topcoat::expr::start("), "{html}");
    assert!(html.contains("::topcoat::expr::end"), "{html}");
}

#[tokio::test]
async fn forwarded_static_bindings_replace_dynamic_bindings() {
    let cx = &Cx::default();
    let previous = attributes! { cx => :value=$("old") :disabled=$(true) };
    let attrs = attributes! {
        cx =>
        (previous)
        :value=(Expr::from("\"<&"))
        :disabled=(Expr::from(false))
        :required=(Expr::from(true))
    };
    let html = view! { cx => <input (attrs)> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("value=\"&quot;<&amp;\""), "{html}");
    assert!(html.contains(" required"), "{html}");
    assert!(!html.contains("disabled"), "{html}");
    assert!(!html.contains("data-topcoat-bind:"), "{html}");
}

#[tokio::test]
async fn forwarded_dynamic_bindings_replace_static_bindings() {
    let cx = &Cx::default();
    let previous = attributes! { cx => :value=(Expr::from("old")) };
    let attrs = attributes! { cx => (previous) :value=$("new") };
    let html = view! { cx => <input (attrs)> }
        .single()
        .await
        .unwrap()
        .render(cx);

    assert!(html.contains("value=\"new\""), "{html}");
    assert!(html.contains("data-topcoat-bind:value="), "{html}");
}
