use std::collections::HashMap;

use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, ViewExt, component, identity::Identity, view},
};

fn empty_cx() -> Cx {
    Cx::default()
}

/// Renders its own identity hash as `label=hash;` for the assertions to
/// parse back out.
#[component]
async fn probe(label: &str) -> Result<impl View> {
    let id = Identity::current().hash();
    Ok(view! { (format!("{label}={id:x};")) })
}

/// Renders the ambiguity error of its identity, or `ok` when there is none.
#[component]
async fn ambiguity() -> Result<impl View> {
    let identity = Identity::try_current();
    Ok(view! {
        match identity {
            Ok(_) => "ok",
            Err(error) => (error.to_string()),
        }
    })
}

#[component]
async fn wrapper(child: Child<'_>) -> Result<impl View> {
    Ok(view! { (child) })
}

/// Parses `label=hash;` pairs out of `rendered`.
fn ids(rendered: &str) -> HashMap<String, String> {
    rendered
        .split_terminator(';')
        .map(|pair| {
            let (label, id) = pair.split_once('=').expect("`label=hash` pair");
            (label.to_owned(), id.to_owned())
        })
        .collect()
}

#[tokio::test]
async fn identities_are_stable_across_renders() {
    let cx = empty_cx();
    let __cx = &cx;
    let render = || async {
        view! { probe(label: "a") }
            .single(__cx)
            .await
            .unwrap()
            .render(__cx)
    };
    assert_eq!(render().await, render().await);
}

#[tokio::test]
async fn sibling_invocations_have_distinct_identities() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! {
        probe(label: "a")
        probe(label: "b")
    }
    .single(__cx)
    .await
    .unwrap()
    .render(__cx);

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

#[tokio::test]
async fn keys_give_each_iteration_its_own_stable_identity() {
    let cx = empty_cx();
    let __cx = &cx;
    let render = |labels: Vec<&'static str>| async move {
        let rendered = view! {
            for label in labels {
                probe(key: label, label: label)
            }
        }
        .single(__cx)
        .await
        .unwrap()
        .render(__cx);
        ids(&rendered)
    };

    let forward = render(vec!["a", "b"]).await;
    let backward = render(vec!["b", "a"]).await;
    assert_ne!(forward["a"], forward["b"]);
    // The identity follows the key, not the position in the loop.
    assert_eq!(forward["a"], backward["a"]);
    assert_eq!(forward["b"], backward["b"]);
}

#[tokio::test]
async fn the_same_key_at_two_sites_stays_distinct() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! {
        probe(key: 1, label: "a")
        probe(key: 1, label: "b")
    }
    .single(__cx)
    .await
    .unwrap()
    .render(__cx);

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

#[tokio::test]
async fn an_unkeyed_component_outside_a_loop_is_unambiguous() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! { ambiguity() }
        .single(__cx)
        .await
        .unwrap()
        .render(__cx);
    assert_eq!(rendered, "ok");
}

#[tokio::test]
async fn an_unkeyed_component_in_a_loop_reports_the_missing_key() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! {
        for _ in 0..1 {
            ambiguity()
        }
    }
    .single(__cx)
    .await
    .unwrap()
    .render(__cx);

    assert!(rendered.contains("`ambiguity`"), "names the invocation");
    assert!(rendered.contains("identity.rs"), "points into this file");
    assert!(rendered.contains("`key`"), "suggests passing a key");
}

#[tokio::test]
async fn an_ambiguous_invocation_poisons_its_children() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! {
        for _ in 0..1 {
            wrapper(ambiguity())
        }
    }
    .single(__cx)
    .await
    .unwrap()
    .render(__cx);

    // The child's error names the outermost invocation missing its key.
    assert!(rendered.contains("`wrapper`"));
}

#[tokio::test]
async fn a_key_resolves_the_children_of_a_repeated_invocation() {
    let cx = empty_cx();
    let __cx = &cx;
    let items = vec!["a", "b"];
    let rendered = view! {
        for item in items {
            wrapper(key: item, probe(label: item))
        }
    }
    .single(__cx)
    .await
    .unwrap()
    .render(__cx);

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}
