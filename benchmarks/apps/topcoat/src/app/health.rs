use topcoat::{Result, context::Cx, router::route};

/// `GET /health`: the readiness probe.
#[route(GET)]
async fn health(cx: &Cx) -> Result<&'static str> {
    Ok("ok")
}
