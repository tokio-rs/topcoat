pub(crate) mod users;

use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{
        Body, Next, StatusCode, error::unauthorized, layer, request::headers, response::Response,
    },
};

// The `Operation` on this layer documents every route under `/api`. Each of
// them inherits the tag, the security requirement, and the 401 response.
#[layer(
    with(Operation::new()
        .tag("users")
        .security("token")
        .response(StatusCode::UNAUTHORIZED, "The token is missing or wrong."))
)]
async fn api(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let authorized = headers(cx)
        .get("authorization")
        .is_some_and(|value| value == "Bearer topcoat-demo");
    if !authorized {
        return Err(unauthorized().into());
    }
    next.run(cx, body).await
}
