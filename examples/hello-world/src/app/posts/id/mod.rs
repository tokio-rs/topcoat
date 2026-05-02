use topcoat::{
    context::Cx,
    router::{page, segment},
    view::{View, view},
};

segment!(kind = Param, parse = uuid::Uuid);

#[page]
async fn post_page(cx: &Cx) -> View {
    view! { "showing post with id: " (id(cx).to_string()) }
}
