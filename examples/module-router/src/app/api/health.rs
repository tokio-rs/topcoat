use topcoat::{Result, context::Cx, router::route};

// Routes also derive their path from the module tree: app::api::health -> GET /api/health.
#[route(GET)]
async fn health(cx: &Cx) -> Result<&'static str> {
    Ok("ok")
}
