//! A page re-run through the runtime's pages route: the request is rewritten
//! into a plain `GET` for the page, and the page's signals resume from the
//! values the client sent.

use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Router, page,
        request::{method, original_method, original_uri, uri},
        to_bytes,
    },
    runtime::{RouterBuilderPageRerunExt, signal},
    view::{View, view},
};

#[page("/search")]
async fn search(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("initial"));
    let current = query.get();
    Ok(view! {
        <p>(method(cx).as_str()) " " (uri(cx).to_string())</p>
        <p>(original_method(cx).as_str()) " " (original_uri(cx).to_string())</p>
        <p>"query: " (current)</p>
    })
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! { <p>"home " (method(cx).as_str())</p> })
}

fn router() -> Router {
    Router::builder()
        .page(search)
        .page(home)
        .page_reruns()
        .build()
}

/// Sends a request through the router, returning the status and body.
async fn send(router: &Router, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut request = http::Request::builder().method(method).uri(path);
    if !body.is_empty() {
        request = request.header("content-type", "application/json");
    }
    let request = request.body(Body::from(body.to_owned())).unwrap();
    let response = router.handle(request).await;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// The id of the last signal declared in `html`.
fn last_signal_id(html: &str) -> &str {
    let declaration = html.rfind("::topcoat::signal(").expect(html);
    let key = "&quot;id&quot;:&quot;";
    let start = html[declaration..].find(key).expect(html) + declaration + key.len();
    let end = html[start..].find("&quot;").expect(html) + start;
    &html[start..end]
}

#[tokio::test]
async fn a_rerun_runs_the_page_as_a_get_for_its_own_url() {
    let router = router();
    let (status, html) = send(&router, "POST", "/_topcoat/runtime/pages/search?q=1", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("<p>GET /search?q=1</p>"), "{html}");
    assert!(
        html.contains("<p>POST /_topcoat/runtime/pages/search?q=1</p>"),
        "{html}"
    );
}

#[tokio::test]
async fn a_rerun_resumes_the_page_signals_from_the_values_it_carries() {
    let router = router();
    let (_, inline) = send(&router, "GET", "/search", "").await;
    assert!(inline.contains("query: initial"), "{inline}");
    let id = last_signal_id(&inline);

    let body = format!(r#"{{"signals":{{"{id}":"shoes"}}}}"#);
    let (status, rerun) = send(&router, "POST", "/_topcoat/runtime/pages/search", &body).await;

    assert_eq!(status, 200, "{rerun}");
    assert!(rerun.contains("query: shoes"), "{rerun}");
    assert_eq!(last_signal_id(&rerun), id, "{inline}\n{rerun}");
    // The page's tracked read renders its dependency marker on both runs.
    assert!(
        rerun.contains(&format!("<!--::topcoat::dep(\"{id}\")-->")),
        "{rerun}"
    );
}

#[tokio::test]
async fn the_root_page_reruns_through_the_bare_prefix() {
    let router = router();
    let (status, html) = send(&router, "POST", "/_topcoat/runtime/pages", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("home GET"), "{html}");
}

#[tokio::test]
async fn a_rerun_of_an_unknown_page_is_not_found() {
    let router = router();
    let (status, _) = send(&router, "POST", "/_topcoat/runtime/pages/missing", "{}").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn a_rerun_needs_a_json_body() {
    let router = router();
    let request = http::Request::builder()
        .method("POST")
        .uri("/_topcoat/runtime/pages/search")
        .body(Body::from("{}"))
        .unwrap();
    let response = router.handle(request).await;
    assert_eq!(response.status().as_u16(), 400);
}
