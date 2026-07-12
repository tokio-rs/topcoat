use topcoat::{
    Result,
    context::Cx,
    router::{Router, Slot, layout, page, uri},
    view::view,
};

mod common;
use common::send;

#[layout("/")]
async fn shell(slot: Slot<'_>) -> Result {
    view! { <main>(slot.await?)</main> }
}

#[layout("/nested")]
async fn section_layout(cx: &Cx, slot: Slot<'_>) -> Result {
    view! { <section data-path=(uri(cx).path())>(slot.await?)</section> }
}

#[page("/nested/inner")]
async fn inner() -> Result {
    view! { "inner" }
}

// A layout used as a component: the child view is passed as the `slot` prop.
#[page("/composed")]
async fn composed() -> Result {
    let content = view! { <p>"content"</p> }?;
    view! { shell(slot: content) }
}

#[tokio::test]
async fn layouts_registered_by_name_wrap_pages() {
    let router = Router::builder()
        .layout(shell)
        .layout(section_layout)
        .page(inner)
        .build();
    let (status, body) = send(&router, "/nested/inner").await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        "<main><section data-path=\"/nested/inner\">inner</section></main>"
    );
}

#[tokio::test]
async fn renders_a_layout_as_a_component() {
    let router = Router::builder().page(composed).build();
    let (status, body) = send(&router, "/composed").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main><p>content</p></main>");
}
