use topcoat::{
    Result,
    context::Cx,
    router::{
        Router, StatusCode,
        error::{NotFoundError, not_found},
        layout, page,
        request::uri,
    },
    view::{ViewHandle, view},
};

mod common;
use common::send;

#[layout("/")]
async fn shell(slot: ViewHandle<'_>) -> Result {
    view! { <main>(slot)</main> }
}

#[layout("/nested")]
async fn section_layout(cx: &Cx, slot: ViewHandle<'_>) -> Result {
    view! { <section data-path=(uri(cx).path())>(slot)</section> }
}

#[page("/nested/inner")]
async fn inner() -> Result {
    view! { "inner" }
}

// A layout used as a component: the child view is passed as the `slot` prop,
// a handle adopted from the `view!` block in argument position.
#[page("/composed")]
async fn composed() -> Result {
    view! { shell(slot: view! { <p>"content"</p> }) }
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

// A catching layout: `live match` consumes the slot handle, handles the
// not-found state in place, and rethrows everything else with `?`.
#[layout("/guard")]
async fn guard(slot: ViewHandle<'_>) -> Result {
    view! {
        <main>
            live match slot {
                Err(error) if error.downcast_ref::<NotFoundError>().is_some() => {
                    (StatusCode::NOT_FOUND)
                    <h1>"nothing here"</h1>
                }
                other => {
                    (other?)
                }
            }
        </main>
    }
}

#[page("/guard/found")]
async fn found() -> Result {
    view! { "found" }
}

#[page("/guard/missing")]
async fn missing() -> Result {
    Err(not_found().into())
}

#[derive(Debug)]
struct BackendDown;

impl std::fmt::Display for BackendDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the backend is down")
    }
}

impl std::error::Error for BackendDown {}

#[page("/guard/broken")]
async fn broken() -> Result {
    Err(BackendDown.into())
}

fn guarded() -> Router {
    Router::builder()
        .layout(guard)
        .page(found)
        .page(missing)
        .page(broken)
        .build()
}

#[tokio::test]
async fn a_layout_splices_the_ok_state() {
    let (status, body) = send(&guarded(), "/guard/found").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<main>found</main>");
}

#[tokio::test]
async fn a_layout_catches_the_error_it_matches() {
    let (status, body) = send(&guarded(), "/guard/missing").await;
    assert_eq!(status, 404);
    assert_eq!(body, "<main><h1>nothing here</h1></main>");
}

#[tokio::test]
async fn the_rethrow_arm_passes_other_errors_along() {
    let (status, _body) = send(&guarded(), "/guard/broken").await;
    assert_eq!(status, 500);
}
