//! Renders every URL that no other route serves as the 404 page, inside the root layout.

use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{error::not_found, page, segment},
};

use crate::components::error_page::PageError;

segment!(kind: CatchAll, rename: "rest");

#[page(*, with(Metadata::new().title(PageError::NotFound.document_title())))]
async fn unknown_page(cx: &Cx) -> Result<()> {
    Err(not_found().into())
}
