use topcoat::{
    Result,
    context::Cx,
    router::{Body, error::rewrite, page},
};

// A rewrite starts a new server-side dispatch without changing the browser URL.
#[page]
pub(crate) async fn rewrite_page(_cx: &Cx) -> Result<()> {
    Err(rewrite("/rewritten", Body::empty()).into())
}
