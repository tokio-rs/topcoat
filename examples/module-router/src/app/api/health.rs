use topcoat::{Result, router::route};

// Routes also derive their path from the module tree: app::api::health -> GET /api/health.
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
