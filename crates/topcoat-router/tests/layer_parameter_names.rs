use std::panic::{AssertUnwindSafe, catch_unwind};

use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Next, Router, layer, response::Response, route},
};

const LAYER_DECLARATION: u32 = line!() + 1;
#[layer("/accounts/{account_id}")]
async fn account_layer(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}

const ROUTE_DECLARATION: u32 = line!() + 1;
#[route(GET "/accounts/{id}")]
async fn account() -> Result<&'static str> {
    Ok("account")
}

#[test]
fn macro_declarations_appear_in_parameter_mismatch_diagnostic() {
    let message = catch_unwind(AssertUnwindSafe(|| {
        let _ = Router::builder()
            .route(account)
            .layer(account_layer)
            .build();
    }))
    .unwrap_err()
    .downcast::<String>()
    .expect("panic payload should be a string");

    assert!(message.contains(&format!(
        "layer `/accounts/{{account_id}}` at {}:{LAYER_DECLARATION}:1",
        file!()
    )));
    assert!(message.contains(&format!(
        "route `/accounts/{{id}}` at {}:{ROUTE_DECLARATION}:1",
        file!()
    )));
}
