use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, view},
};

#[page]
pub(crate) async fn rewritten(cx: &Cx) -> Result<impl View> {
    Ok(view! { <h1>"The rewrite target"</h1> })
}
