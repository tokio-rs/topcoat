use std::time::Duration;

use tokio::{sync::Barrier, time::timeout};
use topcoat::{
    Result,
    context::Cx,
    view::{Component, View, component, pass::Driver, view},
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

/// Runs `fut`, failing instead of hanging when the expected concurrency
/// regresses to sequential awaits.
async fn assert_concurrent<F: Future>(fut: F) -> F::Output {
    timeout(Duration::from_secs(5), fut)
        .await
        .expect("components should render concurrently")
}

#[component]
async fn sibling_meets(barrier: &Barrier) -> Result {
    view! {
        <p>
            meet(barrier: barrier, label: "a")
            "-"
            meet(barrier: barrier, label: "b")
        </p>
    }
}

#[tokio::test]
async fn sibling_components_render_concurrently_in_source_order() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, sibling_meets, barrier = &barrier)
            .await
            .unwrap();

        assert_eq!(html, "<p><i>a</i>-<i>b</i></p>");
    })
    .await;
}

#[component]
async fn loop_meets(barrier: &Barrier) -> Result {
    view! {
        <ul>
            for label in ["a", "b", "c"] {
                <li>meet(key: label, barrier: barrier, label: label)</li>
            }
        </ul>
    }
}

#[tokio::test]
async fn loop_iterations_render_concurrently_in_iteration_order() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(3);
        let html = drive!(&cx, loop_meets, barrier = &barrier).await.unwrap();

        assert_eq!(
            html,
            "<ul><li><i>a</i></li><li><i>b</i></li><li><i>c</i></li></ul>",
        );
    })
    .await;
}

#[component]
async fn branch_meets(barrier: &Barrier) -> Result {
    view! {
        meet(barrier: barrier, label: "always")
        if 1 + 1 == 2 {
            meet(barrier: barrier, label: "sometimes")
        }
    }
}

#[tokio::test]
async fn taken_if_branch_renders_concurrently_with_siblings() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, branch_meets, barrier = &barrier).await.unwrap();

        assert_eq!(html, "<i>always</i><i>sometimes</i>");
    })
    .await;
}

#[component]
async fn arm_meets(barrier: &Barrier, choice: u8) -> Result {
    view! {
        meet(barrier: barrier, label: "always")
        match choice {
            0 => {
                meet(barrier: barrier, label: "zero")
            }
            1 => {
                meet(barrier: barrier, label: "one")
            }
            _ => {
                meet(barrier: barrier, label: "many")
            }
        }
    }
}

#[tokio::test]
async fn taken_match_arm_renders_concurrently_with_siblings() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, arm_meets, barrier = &barrier, choice = 1)
            .await
            .unwrap();

        assert_eq!(html, "<i>always</i><i>one</i>");
    })
    .await;
}

#[component]
async fn wrapper(child: View) -> Result {
    view! { <div>(child?)</div> }
}

#[component]
async fn wrapped_meets(barrier: &Barrier) -> Result {
    view! {
        meet(barrier: barrier, label: "sibling")
        wrapper(meet(barrier: barrier, label: "inner"))
    }
}

#[tokio::test]
async fn child_views_render_concurrently_with_their_parents_siblings() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, wrapped_meets, barrier = &barrier)
            .await
            .unwrap();

        assert_eq!(html, "<i>sibling</i><div><i>inner</i></div>");
    })
    .await;
}

/// Wraps its child once every party has arrived at the barrier.
#[component]
async fn meet_wrapper(barrier: &Barrier, child: View) -> Result {
    barrier.wait().await;
    view! { <div>(child?)</div> }
}

#[component]
async fn self_meets(barrier: &Barrier) -> Result {
    view! { meet_wrapper(barrier: barrier, meet(barrier: barrier, label: "inner")) }
}

#[tokio::test]
async fn a_component_renders_concurrently_with_its_own_child() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, self_meets, barrier = &barrier).await.unwrap();

        assert_eq!(html, "<div><i>inner</i></div>");
    })
    .await;
}

#[component]
async fn deep_meets(barrier: &Barrier) -> Result {
    view! {
        meet_wrapper(
            barrier: barrier,
            meet_wrapper(barrier: barrier, meet(barrier: barrier, label: "deep"))
        )
    }
}

// A component holds its child's render future while its own runs, so the
// nested chain's futures stack up in one place.
#[allow(clippy::large_futures)]
#[tokio::test]
async fn nested_components_render_concurrently_at_every_depth() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(3);
        let html = drive!(&cx, deep_meets, barrier = &barrier).await.unwrap();

        assert_eq!(html, "<div><div><i>deep</i></div></div>");
    })
    .await;
}

#[component]
async fn echo(text: &str) -> Result {
    view! { <b>(text)</b> }
}

#[component]
async fn echoes() -> Result {
    view! {
        let greeting = "hello";
        echo(text: greeting)
        let farewell = "goodbye";
        echo(text: farewell)
    }
}

#[tokio::test]
async fn joined_components_still_read_earlier_local_bindings() {
    let cx = Cx::default();
    let html = drive!(&cx, echoes).await.unwrap();

    assert_eq!(html, "<b>hello</b><b>goodbye</b>");
}

#[component]
async fn interleaved_meets(barrier: &Barrier) -> Result {
    view! {
        <ol>
            for (index, label) in ["a", "b"].into_iter().enumerate() {
                <li value=(index + 1)>
                    meet(key: index, barrier: barrier, label: label)
                    (label.to_uppercase())
                </li>
            }
        </ol>
    }
}

#[tokio::test]
async fn concurrent_loop_interleaves_static_markup_in_order() {
    assert_concurrent(async {
        let cx = Cx::default();
        let barrier = Barrier::new(2);
        let html = drive!(&cx, interleaved_meets, barrier = &barrier)
            .await
            .unwrap();

        assert_eq!(
            html,
            "<ol><li value=\"1\"><i>a</i>A</li><li value=\"2\"><i>b</i>B</li></ol>",
        );
    })
    .await;
}
