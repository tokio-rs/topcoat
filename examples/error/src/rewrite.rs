use topcoat::{
    Result,
    router::{Body, error::rewrite, page},
};

// A rewrite starts a new server-side dispatch without changing the browser URL.
#[page]
pub(crate) async fn rewrite_page() -> Result<()> {
    Err(rewrite("/rewritten", Body::empty()).into())
}
