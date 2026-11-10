mod api;

use topcoat::{
    Result,
    context::Cx,
    openapi::{Document, OpenApi, SecurityScheme},
    router::{module_router, route, router},
};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

// Serves the OpenAPI document. The route has no `Operation` and sits outside
// the documented `/api` layer, so the document does not list it.
#[route(GET "./openapi.json")]
async fn spec(cx: &Cx) -> Result<Document> {
    Ok(OpenApi::new("Users API", "1.0.0")
        .description("A small API for managing users.")
        .tag("users", "Create, list, and look up users.")
        .security_scheme("token", SecurityScheme::bearer())
        .generate(&router(cx)))
}
