use topcoat::{Result, context::Cx, router::{href, page, path_param}, view::{View, view}};

path_param!(pub(crate)
    post_id: u32,
    error = bad_request("Post ID must be a number!"),
);

#[page]
pub(crate) async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<crate::posts::post_id::PostId>(cx)?;

    Ok(view! {
        <h1>
            "Post "
            (post_id)
        </h1>
        <p>"parsed from the {post_id} path segment"</p>
        <p><a href=(href!(crate::posts::posts).query([("page", 1)]))>"all posts"</a></p>
    })
}
