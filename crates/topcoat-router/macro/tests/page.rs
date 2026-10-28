use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, context},
    router::{Body, ModulePage, Page, Router, content::Form, href, page, request::uri, to_bytes},
    view::{View, view},
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
async fn home() -> Result<impl View> {
    Ok(view! { <h1>"home"</h1> })
}

#[derive(Deserialize)]
struct Search {
    q: String,
}

// A page that reads a request body through a destructuring pattern.
#[page("/search")]
async fn search(Form(input): Form<Search>) -> Result<impl View> {
    Ok(view! {
        <p>
            "searching for "
            (input.q)
        </p>
    })
}

#[page("/whoami")]
async fn whoami(cx: &Cx) -> Result<impl View> {
    Ok(view! { <p>(uri(cx).path())</p> })
}

// A page whose body binds its own name: the binding must shadow the
// generated marker.
#[page("/shadowed")]
async fn shadowed() -> Result<impl View> {
    let shadowed = "shadowed";
    Ok(view! { <p>(shadowed)</p> })
}

// A page linking to itself: the marker stays reachable from the page's own
// body, next to a binding of the same name shadowing it.
#[page("/self-linked")]
async fn self_linked() -> Result<impl View> {
    let self_linked = "self-linked";
    Ok(view! { <a href=(href!(self_linked))>(self_linked)</a> })
}

// Pages used as components: called like any component inside `view!`, with a
// request body passed as the already-parsed `body` prop.
#[page("/composed")]
async fn composed() -> Result<impl View> {
    let query = Search {
        q: String::from("topcoat"),
    };
    Ok(view! {
        home()
        search(body: Form(query))
        whoami()
    })
}

// A page serving a method other than the default `GET`.
#[page(POST "/submit")]
async fn submit() -> Result<impl View> {
    Ok(view! { <p>"submitted"</p> })
}

// A page serving several methods at one path.
#[page([GET, POST] "/either")]
async fn either() -> Result<impl View> {
    Ok(view! { <p>"either"</p> })
}

// A page serving every method.
#[page(* "/anything")]
async fn anything() -> Result<impl View> {
    Ok(view! { <p>"anything"</p> })
}

#[derive(Debug, PartialEq)]
struct Title(&'static str);

#[derive(Debug, PartialEq)]
struct Tag(&'static str);

mod presets {
    pub fn tag() -> super::Tag {
        super::Tag("preset")
    }
}

// A page with route context: each `with(...)` adds one value.
#[page(POST "/described", with(Title("described")), with(presets::tag()))]
async fn described() -> Result<impl View> {
    Ok(view! { <p>"described"</p> })
}

// A page repeating a type: the later value shadows the earlier one.
#[page("/retitled", with(Title("first")), with(Title("second")))]
async fn retitled() -> Result<impl View> {
    Ok(view! { <p>"retitled"</p> })
}

// A page reading its own route context during a request.
#[page("/titled", with(Title("titled")))]
async fn titled(cx: &Cx) -> Result<impl View> {
    let title = context::<Title>(cx).0;
    Ok(view! { <p>(title)</p> })
}

// A page with route context but no path, derived by the module router.
#[page(with(Title("module")))]
async fn module_described() -> Result<impl View> {
    Ok(view! { <p>"module"</p> })
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
async fn a_page_links_to_itself() {
    let router = Router::builder().page(self_linked).build();
    let (status, body) = send(&router, "/self-linked").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<a href=\"/self-linked\">self-linked</a>");
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

#[test]
fn a_page_holds_the_values_of_its_context() {
    let context = Page::context(&described);
    assert_eq!(context.get::<Title>(), Some(&Title("described")));
    assert_eq!(context.get::<Tag>(), Some(&Tag("preset")));
}

#[test]
fn a_page_without_context_holds_no_values() {
    assert!(Page::context(&home).get::<Title>().is_none());
}

#[test]
fn a_later_context_value_shadows_an_earlier_one() {
    let context = Page::context(&retitled);
    assert_eq!(context.get::<Title>(), Some(&Title("second")));
    let titles: Vec<_> = context.iter::<Title>().collect();
    assert_eq!(titles, [&Title("second"), &Title("first")]);
}

#[tokio::test]
async fn a_page_reads_its_context_during_a_request() {
    let router = Router::builder().page(titled).build();
    let (status, body) = send(&router, "/titled").await;
    assert_eq!(status, 200);
    assert_eq!(body, "<p>titled</p>");
}

#[test]
fn a_module_page_holds_the_values_of_its_context() {
    let context = ModulePage::context(&module_described);
    assert_eq!(context.get::<Title>(), Some(&Title("module")));
}
