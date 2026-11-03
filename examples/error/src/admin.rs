use topcoat::{
    Result,
    context::Cx,
    router::{error::forbidden, page},
};

// An error constructor converts into the handler's error type.
#[page]
pub(crate) async fn admin(_cx: &Cx) -> Result<()> {
    Err(forbidden().into())
}
