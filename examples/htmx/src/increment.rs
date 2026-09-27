use crate::Counter;
use std::sync::atomic::Ordering;
use topcoat::{Result, context::{Cx, app_context}, htmx::HxResponseTrigger, router::route, view::{ViewHandle, view}};

#[route(POST)]
pub(crate) async fn increment(cx: &Cx) -> Result<(HxResponseTrigger, ViewHandle)> {
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;
    let fragment = view! { cx => <span id="count">(count)</span> }
        .single()
        .await?;

    // The trigger becomes an `HX-Trigger: counted` response header, which
    // fires a `counted` event in the browser.
    Ok((HxResponseTrigger::receive(["counted"]), fragment))
}
