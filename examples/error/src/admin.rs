use topcoat::{
    Result,
    router::{error::forbidden, page},
};

// An error constructor converts into the handler's error type.
#[page]
pub(crate) async fn admin() -> Result<()> {
    Err(forbidden().into())
}
