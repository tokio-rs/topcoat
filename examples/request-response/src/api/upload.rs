use topcoat::{Result, context::Cx, router::{Body, body_limit, route, to_bytes}};

// Body gives the handler the raw stream when it wants to parse bytes itself.
// A raw stream bypasses the body limit; pass body_limit(cx) to keep it.
#[route(POST)]
pub(crate) async fn upload(cx: &Cx, body: Body) -> Result<String> {
    let bytes = to_bytes(body, body_limit(cx)).await?;

    Ok(format!("received {} bytes", bytes.len()))
}
