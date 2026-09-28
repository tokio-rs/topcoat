use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{View, view},
};

module_param!(pub(crate) post_id: u64, error = bad_request);

// ok_or_not_found turns the None into a 404, which the layout's error boundary catches.
#[page]
pub(crate) async fn post(cx: &Cx) -> Result<impl View> {
    let title = match *path_param::<crate::posts::post_id::PostId>(cx)? {
        1 => Some("Hello Topcoat"),
        2 => Some("Error handling"),
        _ => None,
    }
    .ok_or_not_found()?;

    Ok(view! { <h1>(title)</h1> })
}
