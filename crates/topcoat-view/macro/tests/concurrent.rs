use std::time::Duration;

use tokio::{sync::Barrier, time::timeout};
use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

// `view!` lowers component calls to expressions that reference `__cx`. In real
// code that name is supplied by `#[page]`, `#[layout]`, `#[route]`, and
// `#[component]`. These tests stand in for those wrappers by binding it by hand.
fn empty_cx() -> Cx {
    Cx::default()
}

/// Renders its label once every party has arrived at the barrier.
///
/// A test with a barrier sized to the number of `meet` calls only completes
/// when all of them render concurrently; sequential renders deadlock and trip
/// the test's timeout.
#[component]
async fn meet(barrier: &Barrier, label: &str) -> Result {
    barrier.wait().await;
    view! { <i>(label)</i> }
}

/// Runs `fut` inside a view scope, failing instead of hanging when the
/// expected concurrency regresses to sequential awaits.
async fn assert_concurrent<F: Future>(fut: F) -> F::Output {
    timeout(Duration::from_secs(5), topcoat::view::scope(fut))
        .await
        .expect("components should render concurrently")
}

#[tokio::test]
async fn sibling_components_render_concurrently_in_source_order() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let result: Result = view! {
            <p>
                meet(barrier: &barrier, label: "a")
                "-"
                meet(barrier: &barrier, label: "b")
            </p>
        };

        assert_eq!(result.unwrap().render(__cx), "<p><i>a</i>-<i>b</i></p>");
    })
    .await;
}

#[tokio::test]
async fn loop_iterations_render_concurrently_in_iteration_order() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(3);
        // An iteration's future takes ownership of what it captures, so
        // values shared by all iterations are borrowed up front.
        let barrier = &barrier;
        let result: Result = view! {
            <ul>
                for label in ["a", "b", "c"] {
                    <li>meet(barrier: barrier, label: label)</li>
                }
            </ul>
        };

        assert_eq!(
            result.unwrap().render(__cx),
            "<ul><li><i>a</i></li><li><i>b</i></li><li><i>c</i></li></ul>",
        );
    })
    .await;
}

#[tokio::test]
async fn taken_if_branch_renders_concurrently_with_siblings() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let result: Result = view! {
            meet(barrier: &barrier, label: "always")
            if 1 + 1 == 2 {
                meet(barrier: &barrier, label: "sometimes")
            }
        };

        assert_eq!(
            result.unwrap().render(__cx),
            "<i>always</i><i>sometimes</i>"
        );
    })
    .await;
}

#[tokio::test]
async fn taken_match_arm_renders_concurrently_with_siblings() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let choice = 1_u8;
        let result: Result = view! {
            meet(barrier: &barrier, label: "always")
            match choice {
                0 => {
                    meet(barrier: &barrier, label: "zero")
                }
                1 => {
                    meet(barrier: &barrier, label: "one")
                }
                _ => {
                    meet(barrier: &barrier, label: "many")
                }
            }
        };

        assert_eq!(result.unwrap().render(__cx), "<i>always</i><i>one</i>");
    })
    .await;
}

#[component]
async fn wrapper(child: View) -> Result {
    view! { <div>(child)</div> }
}

#[tokio::test]
async fn child_views_render_concurrently_with_their_parents_siblings() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let result: Result = view! {
            meet(barrier: &barrier, label: "sibling")
            wrapper(meet(barrier: &barrier, label: "inner"))
        };

        assert_eq!(
            result.unwrap().render(__cx),
            "<i>sibling</i><div><i>inner</i></div>",
        );
    })
    .await;
}

#[component]
async fn echo(text: &str) -> Result {
    view! { <b>(text)</b> }
}

#[tokio::test]
async fn joined_components_still_read_earlier_local_bindings() {
    topcoat::view::scope(async {
        let cx = empty_cx();
        let __cx = &cx;
        let result: Result = view! {
            let greeting = "hello";
            echo(text: greeting)
            let farewell = "goodbye";
            echo(text: farewell)
        };

        assert_eq!(result.unwrap().render(__cx), "<b>hello</b><b>goodbye</b>",);
    })
    .await;
}

#[tokio::test]
async fn concurrent_loop_interleaves_static_markup_in_order() {
    assert_concurrent(async {
        let cx = empty_cx();
        let __cx = &cx;
        let barrier = Barrier::new(2);
        let barrier = &barrier;
        let result: Result = view! {
            <ol>
                for (index, label) in ["a", "b"].into_iter().enumerate() {
                    <li value=(index + 1)>
                        meet(barrier: barrier, label: label)
                        (label.to_uppercase())
                    </li>
                }
            </ol>
        };

        assert_eq!(
            result.unwrap().render(__cx),
            "<ol><li value=\"1\"><i>a</i>A</li><li value=\"2\"><i>b</i>B</li></ol>",
        );
    })
    .await;
}
