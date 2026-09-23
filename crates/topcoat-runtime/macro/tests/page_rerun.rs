//! Page reruns through the runtime layer.
//!
//! A `POST` with `X-Topcoat-Runtime: true` becomes a `GET` at the same URL.
//! The page's signals resume from the values supplied by the client.

use std::sync::{Arc, Mutex};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, Layer, LayerFuture, Next, Path, Router, RouterBuilder, Slot, layout, page,
        request::{headers, method, original_method, original_uri, uri},
        route, to_bytes,
    },
    runtime::{RUNTIME_HEADER, RouterBuilderRuntimeExt, signal},
    view::{View, view},
};

#[page("/search")]
async fn search(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("initial"));
    let current = query.get();
    Ok(view! {
        <p>
            (method(cx).as_str())
            " "
            (uri(cx).to_string())
        </p>
        <p>
            (original_method(cx).as_str())
            " "
            (original_uri(cx).to_string())
        </p>
        <p>
            "query: "
            (current)
        </p>
        <p>
            "content-type: "
            (headers(cx)
                .get("content-type")
                .map_or("none", |value| value.to_str().unwrap()))
        </p>
    })
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <p>
            "home "
            (method(cx).as_str())
        </p>
    })
}

#[layout("/")]
async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <main>
            "shell "
            (method(cx).as_str())
            " "
            (slot)
        </main>
    })
}

/// Handles ordinary form submissions at the same URL as the search page.
#[route(POST "/search")]
async fn submit() -> Result<&'static str> {
    Ok("submitted")
}

/// Request methods recorded in the order they were received.
type Seen = Mutex<Vec<String>>;

/// A pathless layer that records each request's method.
struct Recorder;

impl Layer for Recorder {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Seen>>(cx)
                .lock()
                .unwrap()
                .push(method(cx).to_string());
            next.run(cx, body).await
        })
    }
}

fn builder() -> RouterBuilder {
    Router::builder()
        .page(search)
        .page(home)
        .layout(shell)
        .route(submit)
}

fn router() -> Router {
    builder().runtime().build()
}

/// Sends a request through the router, returning the status and body.
async fn send(
    router: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut request = http::Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let request = request.body(Body::from(body.to_owned())).unwrap();
    let response = router.handle(request).await;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// Sends a page rerun with the runtime header and supplied JSON body.
async fn rerun(router: &Router, path: &str, body: &str) -> (u16, String) {
    let headers = [
        ("content-type", "application/json"),
        (RUNTIME_HEADER.as_str(), "true"),
    ];
    send(router, "POST", path, &headers, body).await
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
    let (status, html) = rerun(&router, "/search?q=1", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("<p>GET /search?q=1</p>"), "{html}");
    assert!(html.contains("<p>POST /search?q=1</p>"), "{html}");
}

#[tokio::test]
async fn a_rerun_runs_the_page_layouts() {
    let router = router();
    let (status, html) = rerun(&router, "/search", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("<main>shell GET "), "{html}");
}

#[tokio::test]
async fn a_rerun_drops_the_headers_describing_its_envelope() {
    let router = router();
    let (status, html) = rerun(&router, "/search", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("content-type: none"), "{html}");
}

#[tokio::test]
async fn a_rerun_resumes_the_page_signals_from_the_values_it_carries() {
    let router = router();
    let (_, inline) = send(&router, "GET", "/search", &[], "").await;
    assert!(inline.contains("query: initial"), "{inline}");
    let id = last_signal_id(&inline);

    let body = format!(r#"{{"signals":{{"{id}":"shoes"}}}}"#);
    let (status, rerun) = rerun(&router, "/search", &body).await;

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
async fn the_root_page_reruns_at_its_own_url() {
    let router = router();
    let (status, html) = rerun(&router, "/", "{}").await;

    assert_eq!(status, 200, "{html}");
    assert!(html.contains("home GET"), "{html}");
}

#[tokio::test]
async fn a_rerun_of_an_unknown_page_is_not_found() {
    let router = router();
    let (status, _) = rerun(&router, "/missing", "{}").await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn a_rerun_needs_a_json_body() {
    let router = router();
    let headers = [(RUNTIME_HEADER.as_str(), "true")];
    let (status, _) = send(&router, "POST", "/search", &headers, "{}").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn an_unmarked_post_reaches_the_form_handler() {
    let router = router();
    let headers = [("content-type", "application/json")];
    let (status, body) = send(&router, "POST", "/search", &headers, "{}").await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body, "submitted");
}

#[tokio::test]
async fn an_unmarked_post_to_a_get_only_page_is_not_allowed() {
    let router = router();
    let (status, _) = send(&router, "POST", "/", &[], "").await;
    assert_eq!(status, 405);
}

#[tokio::test]
async fn a_cross_site_rerun_is_forbidden() {
    let router = router();
    let headers = [
        ("content-type", "application/json"),
        (RUNTIME_HEADER.as_str(), "true"),
        ("sec-fetch-site", "cross-site"),
    ];
    let (status, _) = send(&router, "POST", "/search", &headers, "{}").await;
    assert_eq!(status, 403);
}

#[tokio::test]
async fn application_layers_registered_before_the_runtime_see_only_the_rewritten_get() {
    let seen = Arc::new(Seen::default());
    let router = builder()
        .layer(Recorder)
        .app_context(Arc::clone(&seen))
        .runtime()
        .build();

    let (status, _) = rerun(&router, "/search", "{}").await;

    assert_eq!(status, 200);
    assert_eq!(*seen.lock().unwrap(), vec!["GET"]);
}
