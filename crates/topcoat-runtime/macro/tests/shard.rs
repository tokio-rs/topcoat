//! What a shard carries between its inline render and a re-render through
//! its endpoint.
//!
//! The scope marker names the identity of the shard invocation, the browser
//! sends it back in a header, and the endpoint installs it before
//! running the shard body, so a signal created inside the body has the same
//! id on both paths. A signal passed as an argument arrives as its id and
//! current value, and the endpoint rebuilds it from them.

use topcoat::{
    Result,
    context::Cx,
    router::{Body, Route, Router, request::IDENTITY_HEADER, response::Response, to_bytes},
    runtime::{Shard, ShardRoute, Signal, shard, signal},
    view::{View, ViewExt, component, view},
};

#[shard]
async fn stateful(cx: &Cx, label: String) -> Result<impl View> {
    let count = signal(cx, || 0.0);
    Ok(view! {
        <p>
            (label)
            " "
            $(count.get())
        </p>
    })
}

#[component]
async fn host(cx: &Cx) -> Result<impl View> {
    let label = signal(cx, || String::from("a"));
    Ok(view! { stateful(label: $(label.get())) })
}

#[shard]
async fn by_signal(query: Signal<String>) -> Result<impl View> {
    Ok(view! { <p>(query.get())</p> })
}

#[component]
async fn signal_host(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("shoes"));
    Ok(view! { by_signal(query: query) })
}

#[shard]
async fn search_results(query: String, limit: usize) -> Result<impl View> {
    Ok(view! {
        <p>
            (query)
            " "
            (limit)
        </p>
    })
}

#[component]
async fn search_host(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, || String::from("shoes"));
    Ok(view! { search_results(query: $(query.get()), limit: 20) })
}

#[shard]
async fn without_arguments(cx: &Cx) -> Result<impl View> {
    let page = signal(cx, || 1usize);
    Ok(view! { <p>(page.get())</p> })
}

/// The shard id and identity arguments of the scope start marker in `html`.
fn scope_marker(html: &str) -> (&str, &str) {
    let start = html.find("::topcoat::shard::start(").expect(html);
    // The marker's quoted arguments alternate with the separators between
    // them: the shard id, then the identity.
    let mut args = html[start..].split('"');
    let shard = args.nth(1).expect(html);
    let identity = args.nth(1).expect(html);
    (shard, identity)
}

/// The id of the last signal declared in `html`.
fn last_signal_id(html: &str) -> &str {
    let declaration = html.rfind("::topcoat::signal(").expect(html);
    let key = "&quot;id&quot;:&quot;";
    let start = html[declaration..].find(key).expect(html) + declaration + key.len();
    let end = html[start..].find("&quot;").expect(html) + start;
    &html[start..end]
}

/// Sends a JSON request through the router, which installs its identity.
async fn endpoint(shard: &'static impl Shard, identity: &str, body: Body) -> Response {
    let route = ShardRoute::new(shard);
    let request = http::Request::builder()
        .method("POST")
        .uri(route.path().as_str())
        .header("content-type", "application/json")
        .header(IDENTITY_HEADER, identity)
        .body(body)
        .unwrap();
    Router::builder().route(route).build().handle(request).await
}

/// Renders `shard` through its endpoint at `identity`, carrying the JSON
/// array `args` of arguments and the JSON object `signals` of signal values.
async fn rerender_with(
    shard: &'static impl Shard,
    identity: &str,
    args: &str,
    signals: &str,
) -> String {
    let body = Body::from(format!(r#"{{"args":{args},"signals":{signals}}}"#));
    let response = endpoint(shard, identity, body).await;
    assert_eq!(response.status(), http::StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Renders the `stateful` shard through its endpoint at `identity`,
/// carrying the JSON object `signals` of signal values.
async fn rerender(identity: &str, signals: &str) -> String {
    rerender_with(&stateful, identity, r#"["a"]"#, signals).await
}

#[tokio::test]
async fn a_shard_without_arguments_accepts_the_browser_request() {
    let cx = &Cx::default();
    let inline = view! { cx => without_arguments() }
        .single()
        .await
        .unwrap()
        .render(cx);
    let (_, identity) = scope_marker(&inline);
    let id = last_signal_id(&inline);
    let signals = format!(
        r#"{{"{id}":{{"t":"usize","bits":{},"v":"2"}}}}"#,
        usize::BITS,
    );
    let rerendered = rerender_with(&without_arguments, identity, "[]", &signals).await;
    assert!(rerendered.contains("<p>2</p>"), "{rerendered}");
}

#[tokio::test]
async fn a_signal_argument_is_read_inline_and_rebuilt_from_its_value() {
    let cx = &Cx::default();
    let inline = view! { cx => signal_host() }
        .single()
        .await
        .unwrap()
        .render(cx);
    let (shard, identity) = scope_marker(&inline);
    assert_eq!(shard, by_signal.id().as_str(), "{inline}");
    assert!(inline.contains("<p>shoes</p>"), "{inline}");
    // The tracked read inside the shard depends on the caller's signal.
    let id = last_signal_id(&inline);
    assert!(
        inline.contains(&format!("::topcoat::dep(\"{id}\")")),
        "{inline}"
    );

    let args = format!(r#"[{{"t":"Signal","id":"{id}","v":"boots"}}]"#);
    let rerendered = rerender_with(&by_signal, identity, &args, "{}").await;

    assert!(rerendered.contains("<p>boots</p>"), "{rerendered}");
    assert!(
        rerendered.contains(&format!("::topcoat::dep(\"{id}\")")),
        "{rerendered}"
    );
}

#[tokio::test]
async fn a_static_argument_keeps_its_javascript_for_rerenders() {
    let cx = &Cx::default();
    let inline = view! { cx => stateful(label: String::from("constant")) }
        .single()
        .await
        .unwrap()
        .render(cx);
    let (_, identity) = scope_marker(&inline);

    assert!(
        inline.contains("cx.hydrate(&quot;constant&quot;)"),
        "{inline}"
    );
    assert!(inline.contains("<p>constant "), "{inline}");

    let rerendered = rerender_with(&stateful, identity, r#"["constant"]"#, "{}").await;
    assert!(rerendered.contains("<p>constant "), "{rerendered}");
}

#[tokio::test]
async fn fixed_and_reactive_arguments_render_together() {
    let cx = &Cx::default();
    let inline = view! { cx => search_host() }
        .single()
        .await
        .unwrap()
        .render(cx);
    let (_, identity) = scope_marker(&inline);

    assert!(inline.contains("<p>shoes 20</p>"), "{inline}");
    assert!(inline.contains(".get()"), "{inline}");
    assert!(inline.contains("&quot;v&quot;:&quot;20&quot;"), "{inline}");

    let args = format!(
        r#"["boots",{{"t":"usize","bits":{},"v":"20"}}]"#,
        usize::BITS,
    );
    let rerendered = rerender_with(&search_results, identity, &args, "{}").await;
    assert!(rerendered.contains("<p>boots 20</p>"), "{rerendered}");
}

#[tokio::test]
async fn a_signal_argument_without_a_value_is_rejected() {
    let body = Body::from(format!(
        r#"{{"args":[{{"t":"Signal","id":"{}"}}]}}"#,
        "0".repeat(32)
    ));
    let response = endpoint(&by_signal, &"A".repeat(22), body).await;
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_rerender_derives_the_same_signal_id_as_the_inline_render() {
    let cx = &Cx::default().keyed("host");
    let inline = view! { cx => host() }.single().await.unwrap().render(cx);
    let (shard, identity) = scope_marker(&inline);
    assert_eq!(shard, stateful.id().as_str(), "{inline}");

    let rerendered = rerender(identity, "{}").await;

    assert_eq!(
        last_signal_id(&rerendered),
        last_signal_id(&inline),
        "{inline}\n{rerendered}"
    );
}

#[tokio::test]
async fn a_rerender_resumes_signals_from_the_values_it_carries() {
    let cx = &Cx::default().keyed("host");
    let inline = view! { cx => host() }.single().await.unwrap().render(cx);
    let (_, identity) = scope_marker(&inline);
    let id = last_signal_id(&inline);
    assert!(inline.contains("&quot;v&quot;:0.0"), "{inline}");

    let rerendered = rerender(identity, &format!(r#"{{"{id}":7.0}}"#)).await;

    assert!(rerendered.contains("&quot;v&quot;:7.0"), "{rerendered}");
    assert!(rerendered.contains("-->7<!--"), "{rerendered}");
}

#[tokio::test]
async fn a_rerender_at_another_identity_derives_another_signal_id() {
    let cx = &Cx::default();
    let inline = view! { cx => host() }.single().await.unwrap().render(cx);

    let rerendered = rerender(&"A".repeat(22), "{}").await;

    assert_ne!(
        last_signal_id(&rerendered),
        last_signal_id(&inline),
        "{inline}\n{rerendered}"
    );
}

#[tokio::test]
async fn a_malformed_identity_is_rejected() {
    let body = Body::from(r#"{"args":["a"]}"#);
    let response = endpoint(&stateful, "not base64", body).await;
    assert_eq!(response.status(), http::StatusCode::BAD_REQUEST);
}
