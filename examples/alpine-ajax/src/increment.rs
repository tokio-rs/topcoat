use crate::{Counter, home};
use std::sync::atomic::Ordering;
use topcoat::{Result, alpine_ajax::ajax_request, context::{Cx, app_context}, router::{error::see_other, href, response::Response, route}, view::view};

#[route(POST)]
pub(crate) async fn increment(cx: &Cx) -> Result<Response> {
    let count = app_context::<Counter>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;

    // An Alpine AJAX request only receives the targeted element.
    if ajax_request(cx) {
        return view! { cx => <span id="count">(count)</span> }
            .single()
            .await?
            .into_response(cx);
    }

    // Without JavaScript, use Post/Redirect/Get and render the complete page.
    see_other(href!(home).resolve(cx)).into_response(cx)
}
