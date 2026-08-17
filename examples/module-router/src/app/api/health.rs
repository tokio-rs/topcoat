use topcoat::Result;

// Routes also derive their path from the module tree: app::api::health -> GET /api/health.
// The attribute is written out in full because importing `route` would collide
// with the handler named after it.
#[topcoat::router::route(GET)]
async fn route() -> Result<&'static str> {
    Ok("ok")
}
