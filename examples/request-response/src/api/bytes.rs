use topcoat::{
    Result,
    context::Cx,
    router::{request::Bytes, route},
};

// Bytes buffers the whole request body for the handler.
#[route(POST)]
pub(crate) async fn read_bytes(_cx: &Cx, body: Bytes) -> Result<String> {
    Ok(format!("received {} bytes", body.len()))
}
