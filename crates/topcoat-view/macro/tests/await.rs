use std::time::Duration;

use tokio::{sync::Barrier, time::timeout};
use topcoat::{
    Result,
    context::Cx,
    view::{View, ViewExt, component, view},
};

// `view!` lowers component calls to expressions that reference `__cx`. In
// real code that name is supplied by `#[page]`, `#[layout]`, and
// `#[component]`. These tests stand in for those wrappers by binding it by
// hand.
fn empty_cx() -> Cx {
    Cx::default()
}

#[component]
async fn echo(text: &str) -> Result<impl View> {
    Ok(view! { <b>(text)</b> })
}

async fn load(values: &[&'static str]) -> Vec<String> {
    tokio::task::yield_now().await;
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[tokio::test]
async fn a_template_may_await_in_a_local_and_borrow_it_afterwards() {
    let cx = empty_cx();
    let __cx = &cx;
    let result = view! {
        <ul>
            let items = load(&["a", "b"]).await;
            for item in &items {
                <li>echo(text: item)</li>
            }
        </ul>
    };

    assert_eq!(
        result.single().await.unwrap().render(__cx),
        "<ul><li><b>a</b></li><li><b>b</b></li></ul>"
    );
}

#[tokio::test]
async fn a_template_may_await_in_a_loop_body_and_in_positions() {
    let cx = empty_cx();
    let __cx = &cx;
    let result = view! {
        for label in ["x", "y"] {
            let loaded = load(&[label]).await;
            <i>(loaded.join(","))</i>
            echo(text: label)
        }
        <p>(load(&["z"]).await.join(","))</p>
        <p class=(load(&["c"]).await.join(","))></p>
    };

    assert_eq!(
        result.single().await.unwrap().render(__cx),
        "<i>x</i><b>x</b><i>y</i><b>y</b><p>z</p><p class=\"c\"></p>"
    );
}

#[tokio::test]
async fn a_template_may_await_in_control_flow_heads() {
    let cx = empty_cx();
    let __cx = &cx;
    let result = view! {
        for item in load(&["a"]).await {
            <i>(item)</i>
        }
        match load(&["b"]).await.first() {
            Some(item) => {
                <b>(item)</b>
            }
            None => {
                "none"
            }
        }
        if let Some(item) = load(&["c"]).await.first() && !item.is_empty() {
            <u>(item)</u>
        }
    };

    assert_eq!(
        result.single().await.unwrap().render(__cx),
        "<i>a</i><b>b</b><u>c</u>"
    );
}

/// Renders its label after awaiting inside its own template, once every
/// party has arrived at the barrier.
#[component]
async fn meet_inside(barrier: &Barrier, label: &str) -> Result<impl View> {
    Ok(view! {
        <i>
            let _ = barrier.wait().await;
            (label)
        </i>
    })
}

#[tokio::test]
async fn sibling_templates_await_concurrently_while_building() {
    timeout(Duration::from_secs(5), async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let result = view! {
            <p>
                meet_inside(barrier: &barrier, label: "a")
                "-"
                meet_inside(barrier: &barrier, label: "b")
            </p>
        };

        assert_eq!(
            result.single().await.unwrap().render(__cx),
            "<p><i>a</i>-<i>b</i></p>"
        );
    })
    .await
    .expect("templates should build concurrently across their awaits");
}
