use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{Form, Router, page, uri},
    view::view,
};

mod common;
use common::send;

#[page("/")]
async fn home() -> Result {
    view! { <h1>"home"</h1> }
}

#[derive(Deserialize)]
struct Search {
    q: String,
}

// A page that reads a request body through a destructuring pattern.
#[page("/search")]
async fn search(Form(input): Form<Search>) -> Result {
    view! {
        <p>
            "searching for "
            (input.q)
        </p>
    }
}

#[page("/whoami")]
async fn whoami(cx: &Cx) -> Result {
    view! { <p>(uri(cx).path())</p> }
}

// A page whose body binds its own name: the binding must shadow the
// generated marker.
#[page("/shadowed")]
async fn shadowed() -> Result {
    let shadowed = "shadowed";
    view! { <p>(shadowed)</p> }
}

// Pages used as components: called like any component inside `view!`, with a
// request body passed as the already-parsed `body` prop.
#[page("/composed")]
async fn composed() -> Result {
    let query = Search {
        q: String::from("topcoat"),
    };
    view! {
        home()
        search(body: Form(query))
        whoami()
    }
}

#[tokio::test]
async fn renders_a_page_registered_by_name() {
    let router = Router::builder().page(home).build();
    let (status, body) = send(&router, "/").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<h1>home</h1>");
}

#[tokio::test]
async fn parses_the_request_body_of_a_registered_page() {
    let router = Router::builder().page(search).build();
    let (status, body) = send(&router, "/search?q=topcoat").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<p>searching for topcoat</p>");
}

#[tokio::test]
async fn a_binding_shadows_the_page_marker() {
    let router = Router::builder().page(shadowed).build();
    let (status, body) = send(&router, "/shadowed").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<p>shadowed</p>");
}

#[tokio::test]
async fn renders_pages_as_components() {
    let router = Router::builder().page(composed).build();
    let (status, body) = send(&router, "/composed").await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        "<h1>home</h1><p>searching for topcoat</p><p>/composed</p>"
    );
}
