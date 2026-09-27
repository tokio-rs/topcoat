pub(crate) mod post_id;

use topcoat::{
    Result,
    context::Cx,
    router::{href, page, query_params},
    view::{View, view},
};

use crate::{PostsQuery, home};

#[page]
pub(crate) async fn posts(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PostsQuery>(cx)?;

    Ok(view! {
        <h1>"Posts"</h1>
        <p>
            "page: "
            (query.page.unwrap_or(1))
        </p>
        <p>
            "search: "
            (query.q.as_deref().unwrap_or("all"))
        </p>
        <p><a href=(href!(home))>"back home"</a></p>
    })
}
