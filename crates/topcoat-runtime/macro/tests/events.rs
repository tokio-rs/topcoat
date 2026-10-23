//! Typed browser event handlers must compile and retain their facade accesses.

use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, ViewExt, component, view},
};

#[component]
async fn event_handlers(cx: &Cx) -> Result<impl View> {
    let destination = signal(cx, String::new);
    let previous = signal(cx, String::new);
    let open = signal(cx, || false);
    Ok(view! {
        <input
            @focusout=$(|e: Event| {
                if e.related_target.is_some() {
                    destination.set(e.related_target.unwrap().id);
                } else {
                    destination.set("".to_owned());
                }
            })
            @focusin=$(|e: Event| {
                if e.related_target.is_none() {
                    destination.set("".to_owned());
                } else {
                    destination.set(e.related_target.expect("related target").id);
                }
            })
            @keydown=$(|e: Event| {
                if !e.is_composing {
                    if e.key == "Escape" {
                        e.prevent_default();
                        open.set(false);
                    }
                }
            })
        >
        <div
            popover="auto"
            @toggle=$(|e: Event| {
                if e.old_state.is_some() {
                    previous.set(e.old_state.unwrap());
                }
                if e.new_state.is_some() {
                    open.set(e.new_state.unwrap() == "open");
                }
            })
        ></div>
    })
}

#[tokio::test]
async fn optional_event_fields_reach_browser_handlers() {
    let cx = &Cx::default();
    let html = view! { cx => event_handlers() }
        .single()
        .await
        .unwrap()
        .render(cx);

    for event in ["focusout", "focusin", "keydown", "toggle"] {
        assert!(
            html.contains(&format!("data-topcoat-on:{event}=")),
            "{html}"
        );
    }
    for access in [
        ".related_target.is_some()",
        ".related_target.is_none()",
        ".related_target.unwrap().id",
        ".related_target.expect(",
        ".old_state.unwrap()",
        ".new_state.unwrap()",
        ".is_composing",
        ".key",
        ".prevent_default()",
    ] {
        assert!(html.contains(access), "missing {access}: {html}");
    }
}
