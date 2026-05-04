use topcoat::{
    context::Cx,
    router::{Result, page, query_params},
    view::view,
};

mod id;

#[query_params]
struct PageQuery {
    page: Option<u32>,
}

#[page]
async fn posts(cx: &Cx) -> Result {
    view! {
        <div>
            "currently on page: "
            (PageQuery::of(cx).as_ref().unwrap().page)
        </div>
    }
}
