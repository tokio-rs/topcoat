use topcoat::{
    Result,
    context::Cx,
    router::{Layout, ModuleLayout, Router, Slot, layout, page, request::uri},
    view::{View, view},
};

mod common;
use common::send;

#[layout("/")]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <main>(slot)</main> })
}

#[layout("/nested")]
async fn section_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <section data-path=(uri(cx).path())>(slot)</section> })
}

#[page("/nested/inner")]
async fn inner() -> Result<impl View> {
    Ok(view! { "inner" })
}

// A layout used as a component: the child view is passed as the `slot` prop.
#[page("/composed")]
async fn composed() -> Result<impl View> {
    let content = view! { <p>"content"</p> };
    Ok(view! { shell(slot: Slot::new(content)) })
}

#[derive(Debug, PartialEq)]
struct Title(&'static str);

// A layout with route context: each `with(...)` adds one value.
#[layout("/admin", with(Title("admin")))]
async fn admin(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <main>(slot)</main> })
}

// A layout with route context but no path, derived by the module router.
#[layout(with(Title("module")))]
async fn module_admin(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <main>(slot)</main> })
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

#[test]
fn a_layout_holds_the_values_of_its_context() {
    let context = Layout::context(&admin);
    assert_eq!(context.get::<Title>(), Some(&Title("admin")));
}

#[test]
fn a_layout_without_context_holds_no_values() {
    assert!(Layout::context(&shell).get::<Title>().is_none());
}

#[test]
fn a_module_layout_holds_the_values_of_its_context() {
    let context = ModuleLayout::context(&module_admin);
    assert_eq!(context.get::<Title>(), Some(&Title("module")));
}
