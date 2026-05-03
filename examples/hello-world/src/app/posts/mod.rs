use topcoat::{
    context::Cx,
    router::{page, query_params},
    view::{View, view},
};

mod id;

#[query_params]
struct PageQuery {
    page: Option<u32>,
}

#[page]
async fn posts(cx: &Cx) -> View {
    view! {
        <div>"currently on page: " (PageQuery::of(cx).as_ref().unwrap().page)</div>
    }
}
