use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
};

use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Layer, Next, Route, Router, layer, response::Response, route},
};

const LAYER_DECLARATION_START: u32 = line!() + 1;
#[layer("/accounts/{account_id}")]
async fn account_layer(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    next.run(cx, body).await
}
const LAYER_DECLARATION_END: u32 = line!() - 1;

const ROUTE_DECLARATION_START: u32 = line!() + 1;
#[route(GET "/accounts/{id}")]
async fn account() -> Result<&'static str> {
    Ok("account")
}
const ROUTE_DECLARATION_END: u32 = line!() - 1;

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).into())
        })
        .expect("panic payload should be a string")
}

#[test]
fn macro_declarations_appear_in_parameter_mismatch_diagnostic() {
    let layer_location = account_layer
        .source_location()
        .expect("macro-generated layer should retain its declaration site");
    let route_location = account
        .source_location()
        .expect("macro-generated route should retain its declaration site");

    assert_eq!(layer_location.file(), file!());
    assert!((LAYER_DECLARATION_START..=LAYER_DECLARATION_END).contains(&layer_location.line()));
    assert_eq!(route_location.file(), file!());
    assert!((ROUTE_DECLARATION_START..=ROUTE_DECLARATION_END).contains(&route_location.line()));

    let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
        Router::builder()
            .route(account)
            .layer(account_layer)
            .build()
    })) else {
        panic!("parameter name mismatch should panic");
    };
    let message = panic_message(&*payload);

    assert!(message.contains(&format!(
        "layer `/accounts/{{account_id}}` at {layer_location}"
    )));
    assert!(message.contains(&format!("route `/accounts/{{id}}` at {route_location}")));
}
