use topcoat::{
    Result,
    context::Cx,
    router::{content::RawForm, route},
};

// RawForm yields the urlencoded bytes without deserializing them.
#[route(POST)]
pub(crate) async fn raw_form(_cx: &Cx, RawForm(bytes): RawForm) -> Result<String> {
    Ok(format!("received {} bytes of form data", bytes.len()))
}
