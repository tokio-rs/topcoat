use std::{collections::HashMap, future::poll_fn, pin::pin};

use topcoat::{
    Result,
    context::{Cx, identity, try_identity},
    view::{Child, View, ViewExt, component, emit, live, view},
};

fn empty_cx() -> Cx {
    Cx::default()
}

/// Renders its own identity hash as `label=hash;` for the assertions to
/// parse back out.
#[component]
async fn probe(cx: &Cx, label: &str) -> Result<impl View> {
    let id = identity(cx).hash();
    tokio::task::yield_now().await;
    assert_eq!(identity(cx).hash(), id);
    Ok(view! { (format!("{label}={id:x};")) })
}

/// Renders the ambiguity error of its identity, or `ok` when there is none.
#[component]
async fn ambiguity(cx: &Cx) -> Result<impl View> {
    let identity = try_identity(cx);
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
            .single()
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
    .single()
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
            #[key(label)]
            for label in labels {
                probe(label: label)
            }
        }
        .single()
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
        #[key(item)]
        for item in [1] {
            probe(label: "a")
        }
        #[key(item)]
        for item in [1] {
            probe(label: "b")
        }
    }
    .single()
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
    let rendered = view! { ambiguity() }.single().await.unwrap().render(__cx);
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
    .single()
    .await
    .unwrap()
    .render(__cx);

    assert!(rendered.contains("`for` loop"), "names the loop");
    assert!(rendered.contains("identity.rs"), "points into this file");
    assert!(rendered.contains("#[key(...)]"), "suggests a loop key");
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
    .single()
    .await
    .unwrap()
    .render(__cx);

    assert!(rendered.contains("`for` loop"));
}

#[tokio::test]
async fn a_key_resolves_the_children_of_a_repeated_invocation() {
    let cx = empty_cx();
    let __cx = &cx;
    let items = vec!["a", "b"];
    let rendered = view! {
        #[key(item)]
        for item in items {
            wrapper(probe(label: item))
        }
    }
    .single()
    .await
    .unwrap()
    .render(__cx);

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

/// Invokes `probe` from its own template rather than as child content.
#[component]
async fn parent(label: &str) -> Result<impl View> {
    Ok(view! { probe(label: label) })
}

#[tokio::test]
async fn a_key_resolves_the_template_of_a_repeated_invocation() {
    let cx = empty_cx();
    let __cx = &cx;
    let items = vec!["a", "b"];
    let rendered = view! {
        #[key(item)]
        for item in items {
            parent(label: item)
        }
    }
    .single()
    .await
    .unwrap()
    .render(__cx);

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

#[tokio::test]
async fn an_explicit_context_inside_a_loop_keeps_its_own_identity() {
    let cx = empty_cx();
    let __cx = &cx;
    let outer = __cx;
    let rendered = view! {
        for _ in [0] {
            (identity(outer).to_string())
        }
    }
    .single()
    .await
    .unwrap()
    .render(__cx);
    assert_eq!(rendered, identity(__cx).to_string());
}

#[tokio::test]
async fn an_inner_key_preserves_outer_ambiguity() {
    let cx = empty_cx();
    let __cx = &cx;
    let rendered = view! {
        for _ in [0] {
            #[key(item)]
            for item in [1] {
                ambiguity()
            }
        }
    }
    .single()
    .await
    .unwrap()
    .render(__cx);
    assert!(rendered.contains("`for` loop"));
}

#[tokio::test]
async fn a_key_borrows_the_item_and_is_evaluated_once_per_iteration() {
    let cx = empty_cx();
    let __cx = &cx;
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let calls = &calls;
    let rendered = view! {
        #[key({ calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed); &item })]
        for item in [String::from("a"), String::from("b")] {
            probe(label: &item)
        }
    }
    .single()
    .await
    .unwrap()
    .render(__cx);
    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 2);
}

fn region_ids(html: &str) -> Vec<&str> {
    html.split("<!--::topcoat::region::start(")
        .skip(1)
        .map(|part| {
            let (id, _) = part.split_once(")-->").unwrap();
            assert_eq!(id.len(), 32);
            assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()));
            id
        })
        .collect()
}

#[tokio::test]
async fn sibling_region_ids_do_not_depend_on_completion_order() {
    let cx = &Cx::default();
    let mut previous = None;
    for slow_first in [false, true] {
        let mut view = pin!(view! {
            cx =>
            (live! {
                if slow_first {
                    tokio::task::yield_now().await;
                }
                emit! { <p>"a"</p> }?;
                emit! { <p>"a updated"</p> }
            })
            (live! {
                if !slow_first {
                    tokio::task::yield_now().await;
                }
                emit! { <p>"b"</p> }?;
                emit! { <p>"b updated"</p> }
            })
        });
        let first = poll_fn(|cx| view.as_mut().poll_first(cx)).await.unwrap();
        let html = first.content.render(cx);
        let ids = region_ids(&html);
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);

        let mut swaps = HashMap::new();
        while let Some(swap) = poll_fn(|cx| view.as_mut().poll_swap(cx)).await.unwrap() {
            swaps.insert(swap.region.to_string(), swap.replacement.render(cx));
        }
        assert_eq!(swaps.len(), 2);
        assert_eq!(swaps[ids[0]], "<p>a updated</p>");
        assert_eq!(swaps[ids[1]], "<p>b updated</p>");
        if let Some(previous) = &previous {
            assert_eq!(&html, previous);
        }
        previous = Some(html);
    }
}

#[component]
async fn updating(label: &str) -> Result<impl View> {
    Ok(live! {
        emit! { (label) }?;
        emit! { "updated" }
    })
}

#[tokio::test]
async fn region_ids_follow_component_keys_when_reordered() {
    let cx = &Cx::default();
    let render = |labels: [&'static str; 2]| async move {
        view! {
            cx =>
            #[key(label)]
            for label in labels {
                updating(label: label)
            }
        }
        .first()
        .await
        .unwrap()
        .render(cx)
    };
    let forward = render(["a", "b"]).await;
    let backward = render(["b", "a"]).await;
    let forward = region_ids(&forward);
    let backward = region_ids(&backward);
    assert_eq!(forward.len(), 2);
    assert_ne!(forward[0], forward[1]);
    assert_eq!(forward[0], backward[1]);
    assert_eq!(forward[1], backward[0]);
}
