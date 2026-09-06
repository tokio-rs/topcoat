//! The identity a shard carries between its inline render and a re-render
//! through its endpoint.
//!
//! The scope marker names the identity of the shard invocation, the browser
//! sends it back in a header, and the endpoint installs it before
//! running the shard body, so a signal created inside the body has the same
//! id on both paths.

use topcoat::{
    Result,
    context::{Cx, CxTestBuilder},
    router::{Body, request::IDENTITY_HEADER},
    runtime::{Shard, shard, signal},
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

/// The path and identity arguments of the scope start marker in `html`.
fn scope_marker(html: &str) -> (&str, &str) {
    let start = html.find("::topcoat::scope::start(").expect(html);
    // The marker's quoted arguments alternate with the separators between
    // them: the scope id, the path, then the identity.
    let mut args = html[start..].split('"');
    let path = args.nth(3).expect(html);
    let identity = args.nth(1).expect(html);
    (path, identity)
}

/// The id of the last signal declared in `html`.
fn last_signal_id(html: &str) -> &str {
    let declaration = html.rfind("::topcoat::signal(").expect(html);
    let key = "&quot;id&quot;:&quot;";
    let start = html[declaration..].find(key).expect(html) + declaration + key.len();
    let end = html[start..].find("&quot;").expect(html) + start;
    &html[start..end]
}

/// Builds the context of a JSON request to the shard endpoint naming
/// `identity` in the identity header.
fn endpoint_cx(identity: &str) -> Cx {
    let (parts, ()) = http::Request::builder()
        .header("content-type", "application/json")
        .header(IDENTITY_HEADER, identity)
        .body(())
        .unwrap()
        .into_parts();
    CxTestBuilder::new().request_context(parts).build()
}

/// Renders the shard through its endpoint at `identity`, carrying the JSON
/// object `signals` of signal values.
async fn rerender(identity: &str, signals: &str) -> String {
    let cx = &endpoint_cx(identity);
    let body = Body::from(format!(r#"{{"args":["a"],"signals":{signals}}}"#));
    stateful.render(cx, body).await.unwrap().render(cx)
}

#[tokio::test]
async fn a_rerender_derives_the_same_signal_id_as_the_inline_render() {
    let cx = &Cx::default();
    let inline = view! { cx => host() }.single().await.unwrap().render(cx);
    let (path, identity) = scope_marker(&inline);
    assert_eq!(
        path,
        format!("/_topcoat/shards/{}", stateful.id().as_str()),
        "{inline}"
    );

    let rerendered = rerender(identity, "{}").await;

    assert_eq!(
        last_signal_id(&rerendered),
        last_signal_id(&inline),
        "{inline}\n{rerendered}"
    );
}

#[tokio::test]
async fn a_rerender_resumes_signals_from_the_values_it_carries() {
    let cx = &Cx::default();
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
    let cx = &endpoint_cx("not base64");
    let body = Body::from(r#"{"args":["a"]}"#);
    assert!(stateful.render(cx, body).await.is_err());
}
