use std::collections::HashMap;

use topcoat::{
    Result,
    context::Cx,
    view::{Component, View, component, identity::Identity, pass::Driver, view},
};

/// Drives a component as its own page and returns the final HTML.
async fn render<C>(cx: &Cx, marker: C, props: C::Props) -> Result<String>
where
    C: Component,
{
    let fut = C::render(marker, cx.detach(), props);
    let mut driver = Driver::new(cx.detach(), fut);
    driver.render_blocking().await
}

/// Renders a component marker with `prop = value` arguments.
macro_rules! drive {
    ($cx:expr, $c:ident $(, $prop:ident = $value:expr)* $(,)?) => {
        render($cx, $c::default(), $c::props_builder()$(.$prop($value))*.build())
    };
}

/// Renders its own identity hash as `label=hash;` for the assertions to
/// parse back out.
#[component]
async fn probe(label: &str) -> Result {
    let id = Identity::current().hash();
    view! { (format!("{label}={id:x};")) }
}

#[component]
async fn single_probe() -> Result {
    view! { probe(label: "a") }
}

#[component]
async fn sibling_probes() -> Result {
    view! {
        probe(label: "a")
        probe(label: "b")
    }
}

#[component]
async fn keyed_probes(labels: Vec<&'static str>) -> Result {
    view! {
        for label in labels.iter().copied() {
            probe(key: label, label: label)
        }
    }
}

#[component]
async fn same_key_two_sites() -> Result {
    view! {
        probe(key: 1, label: "a")
        probe(key: 1, label: "b")
    }
}

#[component]
async fn wrapper(child: View) -> Result {
    view! { (child?) }
}

#[component]
async fn keyed_wrappers(items: Vec<&'static str>) -> Result {
    view! {
        for item in items.iter().copied() {
            wrapper(key: item, probe(label: item))
        }
    }
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
    let cx = Cx::default();
    let first = drive!(&cx, single_probe).await.unwrap();
    let second = drive!(&cx, single_probe).await.unwrap();
    assert_eq!(first, second);
}

#[tokio::test]
async fn sibling_invocations_have_distinct_identities() {
    let cx = Cx::default();
    let rendered = drive!(&cx, sibling_probes).await.unwrap();

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

#[tokio::test]
async fn keys_give_each_iteration_its_own_stable_identity() {
    let cx = Cx::default();
    let forward = ids(&drive!(&cx, keyed_probes, labels = vec!["a", "b"])
        .await
        .unwrap());
    let backward = ids(&drive!(&cx, keyed_probes, labels = vec!["b", "a"])
        .await
        .unwrap());

    assert_ne!(forward["a"], forward["b"]);
    // The identity follows the key, not the position in the loop.
    assert_eq!(forward["a"], backward["a"]);
    assert_eq!(forward["b"], backward["b"]);
}

#[tokio::test]
async fn the_same_key_at_two_sites_stays_distinct() {
    let cx = Cx::default();
    let rendered = drive!(&cx, same_key_two_sites).await.unwrap();

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}

#[tokio::test]
async fn a_key_resolves_the_children_of_a_repeated_invocation() {
    let cx = Cx::default();
    let rendered = drive!(&cx, keyed_wrappers, items = vec!["a", "b"])
        .await
        .unwrap();

    let ids = ids(&rendered);
    assert_ne!(ids["a"], ids["b"]);
}
