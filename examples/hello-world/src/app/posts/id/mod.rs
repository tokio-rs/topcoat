use topcoat::{
    context::Cx,
    router::{page, path_param},
    view::{View, view},
};

#[path_param]
struct PostId(uuid::Uuid);

#[page]
async fn post_page(cx: &Cx) -> View {
    let post_id = PostId::of(cx).as_ref().unwrap();
    view! { "showing post with id: " (post_id.to_string()) }
}
