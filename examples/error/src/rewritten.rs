use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

#[page]
pub(crate) async fn rewritten() -> Result<impl View> {
    Ok(view! { <h1>"The rewrite target"</h1> })
}
