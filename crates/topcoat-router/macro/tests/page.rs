use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Form, Router, page, to_bytes, uri},
    view::view,
};

mod common;
use common::send;

/// Like [`send`], but with an explicit request method.
async fn send_as(router: &Router, method: &str, path: &str) -> (u16, String) {
    let request = http::Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = router.handle(request).await;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

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

// A page serving a method other than the default `GET`.
#[page(POST "/submit")]
async fn submit() -> Result {
    view! { <p>"submitted"</p> }
}

// A page serving several methods at one path.
#[page([GET, POST] "/either")]
async fn either() -> Result {
    view! { <p>"either"</p> }
}

// A page serving every method.
#[page(* "/anything")]
async fn anything() -> Result {
    view! { <p>"anything"</p> }
}

#[tokio::test]
async fn renders_a_page_registered_by_name() {
    let router = Router::builder().page(home).build();
    let (status, body) = send(&router, "/").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<h1>home</h1>");
}

#[tokio::test]
async fn a_page_serves_get_by_default() {
    let router = Router::builder().page(home).build();
    assert_eq!(send_as(&router, "POST", "/").await.0, 405);
}

#[tokio::test]
async fn a_page_can_declare_another_method() {
    let router = Router::builder().page(submit).build();
    let (status, body) = send_as(&router, "POST", "/submit").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<p>submitted</p>");
    assert_eq!(send_as(&router, "GET", "/submit").await.0, 405);
}

#[tokio::test]
async fn a_page_can_declare_a_method_list() {
    let router = Router::builder().page(either).build();
    for method in ["GET", "POST"] {
        let (status, body) = send_as(&router, method, "/either").await;
        assert_eq!(status, 200);
        assert_eq!(body, "<p>either</p>");
    }
}

#[tokio::test]
async fn a_star_page_serves_every_method() {
    let router = Router::builder().page(anything).build();
    for method in ["GET", "POST", "PUT", "DELETE", "PATCH"] {
        assert_eq!(
            send_as(&router, method, "/anything").await,
            (200, "<p>anything</p>".into())
        );
    }
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
