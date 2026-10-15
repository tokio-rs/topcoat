use topcoat::{
    Result,
    context::Cx,
    runtime::{EffectContext, Event, effect, expr, signal},
    view::{View, ViewExt, attributes, component, view},
};

#[component]
async fn panel(cx: &Cx) -> Result<impl View> {
    let ready = signal(cx, || false);
    let mounted = effect(expr!(|scope: EffectContext| {
        ready.set(true);
        scope.on_cleanup(|| ready.set(false));
    }));
    let attrs = attributes! { (mounted) };
    let reset = effect(expr!(|scope: EffectContext| {
        scope.listen(scope.target(), "reset", |event: Event| {
            event.prevent_default();
            ready.set(false);
        });
    }));
    let browser_only = effect(expr!(|scope: EffectContext| {
        scope.on_cleanup(|| {
            raw!("window.cleaned = true");
        });
        if scope.is_active() {
            raw!("${scope}.element.focus()");
        }
    }));
    // Effect bodies do not run while the server constructs or renders the view.
    assert!(!ready.get_untracked());
    Ok(view! {
        <section (attrs)>
            <form (reset)></form>
            <button (browser_only)>"Action"</button>
            <output>$(ready.get())</output>
        </section>
    })
}

#[tokio::test]
async fn effects_compile_to_attributes_without_running_on_the_server() {
    let cx = &Cx::default();
    let html = view! { cx => panel() }.single().await.unwrap().render(cx);
    assert_eq!(html.matches("data-topcoat-effect=").count(), 3, "{html}");
    assert!(html.contains(".listen("), "{html}");
    assert!(html.contains(".on_cleanup("), "{html}");
    assert!(html.contains(".is_active()"), "{html}");
    assert!(html.contains(".element.focus()"), "{html}");
    assert!(html.contains(">false<!--"), "{html}");
    assert!(!html.contains("::topcoat::dep("), "{html}");
}
