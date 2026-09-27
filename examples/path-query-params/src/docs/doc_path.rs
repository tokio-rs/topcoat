use topcoat::{
    Result,
    context::Cx,
    router::{href, page, path_param},
    view::{View, view},
};

use crate::home;

path_param!(pub(crate) *doc_path);

#[page]
pub(crate) async fn document(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"Documentation path"</h1>
        <ul>
            for segment in path_param::<crate::docs::doc_path::DocPath>(cx) {
                <li>(segment)</li>
            }
        </ul>
        <p><a href=(href!(home))>"back home"</a></p>
    })
}
