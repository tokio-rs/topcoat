//! Integration tests for the code the `view!` and `#[component]` macros
//! generate on top of the live render runtime: reactive nodes for `live`
//! constructs, fused invocations, view-handle props, pending-adopted `view!`
//! bindings, and batched loops. The driver polls the render as one task and
//! reads the document chunk by chunk between polls.

use std::{
    fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use topcoat::{
    Result,
    view::{Component, Deferred, View, ViewHandle, component, view},
};
use topcoat_core::context::Cx;
use topcoat_view::live::test_support::{Channel, TestRender, channel};

/// A future resolved by hand from the test: pending, with no waker, until
/// its trigger is pulled.
struct Manual<T> {
    value: Arc<Mutex<Option<T>>>,
}

/// The test's side of a [`Manual`] future.
struct Trigger<T> {
    value: Arc<Mutex<Option<T>>>,
}

fn manual<T: Send>() -> (Trigger<T>, Manual<T>) {
    let value = Arc::new(Mutex::new(None));
    (
        Trigger {
            value: value.clone(),
        },
        Manual { value },
    )
}

impl<T> Trigger<T> {
    fn resolve(&self, value: T) {
        *self.value.lock().expect("the trigger lock was poisoned") = Some(value);
    }
}

impl<T: Send> Future for Manual<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _task: &mut Context<'_>) -> Poll<T> {
        match self
            .value
            .lock()
            .expect("the trigger lock was poisoned")
            .take()
        {
            Some(value) => Poll::Ready(value),
            None => Poll::Pending,
        }
    }
}

/// The error a failing load in these tests reports.
#[derive(Debug)]
struct BackendDown;

impl fmt::Display for BackendDown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the backend is down")
    }
}

impl std::error::Error for BackendDown {}

/// Starts one component as the root of a [`TestRender`].
macro_rules! render {
    ($component:ident $(, $prop:ident: $value:expr)* $(,)?) => {
        TestRender::new(|fill| {
            let cx = Cx::default();
            let props = $component::props_builder()$(.$prop($value))*.build();
            async move { $component.render(&cx, props, fill).await }
        })
    };
}

#[component]
async fn drink_grid(drinks: Manual<Result<Vec<String>>>) -> Result {
    view! {
        <div>
            live match defer(drinks) {
                Deferred::Pending => {
                    <p>"drinks loading"</p>
                }
                Deferred::Ready(drinks) => {
                    for drink in drinks? {
                        <li>(drink)</li>
                    }
                }
            }
        </div>
    }
}

#[test]
fn a_deferred_page_streams_through_the_render() {
    let cx = Cx::default();
    let (drinks, drinks_future) = manual::<Result<Vec<String>>>();
    let mut render = render!(drink_grid, drinks: drinks_future);

    // The first chunk carries the shell and the pending arm's skeleton.
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<div><p>drinks loading</p></div>"),
    );

    // The deferred data arrives: the ready arm swaps in and the render
    // completes.
    drinks.resolve(Ok(vec!["espresso".to_string(), "mojito".to_string()]));
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<div><li>espresso</li><li>mojito</li></div>"),
    );
}

#[test]
fn a_deferred_arm_error_climbs_to_the_root() {
    let cx = Cx::default();
    let (drinks, drinks_future) = manual::<Result<Vec<String>>>();
    let mut render = render!(drink_grid, drinks: drinks_future);

    // The first paint goes out before anything fails.
    assert!(render.settle().is_pending());
    assert!(render.html(&cx).is_some());

    // The ready arm's `?` fails the node and climbs to the root with its
    // type intact.
    drinks.resolve(Err(BackendDown.into()));
    let Poll::Ready(Err(error)) = render.settle() else {
        panic!("the error did not climb to the root");
    };
    assert!(error.downcast_ref::<BackendDown>().is_some());
}

#[component]
async fn failing() -> Result {
    Err(BackendDown.into())
}

#[component]
async fn catcher() -> Result {
    view! {
        live match failing() {
            Ok(content) => {
                (content)
            }
            Err(_) => {
                <p>"the menu is unavailable"</p>
            }
        }
    }
}

#[test]
fn a_live_match_on_an_invocation_catches_its_error() {
    let cx = Cx::default();
    let mut render = render!(catcher);

    // The component fails before delivering; the `Err` arm renders in its
    // place and the error never climbs past the construct.
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<p>the menu is unavailable</p>"),
    );
}

#[component]
async fn greeting() -> Result {
    view! { <h1>"hello"</h1> }
}

#[component]
async fn shown() -> Result {
    view! {
        live match greeting() {
            Ok(content) => {
                (content)
            }
            Err(_) => {
                <p>"unavailable"</p>
            }
        }
    }
}

#[test]
fn a_live_match_on_an_invocation_shows_the_delivered_view() {
    let cx = Cx::default();
    let mut render = render!(shown);

    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(render.html(&cx).as_deref(), Some("<h1>hello</h1>"));
}

#[component]
async fn card(title: ViewHandle<'_>, child: View) -> Result {
    view! {
        <section>
            <header>(title)</header>
            (child)
        </section>
    }
}

#[component]
async fn card_page() -> Result {
    view! { card(title: view! { <h2>"Orders"</h2> }, <p>"body"</p>) }
}

#[test]
fn a_view_handle_prop_is_spliced_by_the_wrapper() {
    let cx = Cx::default();
    let mut render = render!(card_page);

    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<section><header><h2>Orders</h2></header><p>body</p></section>"),
    );
}

#[component]
async fn shared(states: Channel<&'static str>) -> Result {
    // A mid-body `view!` binding: the handle is created here, adopted by
    // the tail expansion, and its one render is shared by both arms.
    let feed = view! { <em>"the feed"</em> };

    view! {
        live match states {
            "first" => {
                <div>
                    "first "
                    (feed.clone())
                </div>
            }
            _ => {
                <div>
                    "second "
                    (feed.clone())
                </div>
            }
        }
    }
}

#[test]
fn a_bound_view_is_shared_across_arm_swaps() {
    let cx = Cx::default();
    let (states, state_feed) = channel::<&'static str>();

    states.send("first");
    let mut render = render!(shared, states: state_feed);

    assert!(render.settle().is_pending());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<div>first <em>the feed</em></div>"),
    );

    // A new state swaps the arm; the new arm's splice of the same handle is
    // served from the cell's cache.
    states.send("other");
    assert!(render.settle().is_pending());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<div>second <em>the feed</em></div>"),
    );

    states.close();
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
}

#[component]
async fn badge(count: Manual<Result<u32>>) -> Result {
    view! {
        <nav>
            live if let Deferred::Ready(count) = defer(count) {
                <span>(count?)</span>
            }
        </nav>
    }
}

#[test]
fn live_if_let_renders_nothing_until_the_state_matches() {
    let cx = Cx::default();
    let (count, count_future) = manual::<Result<u32>>();
    let mut render = render!(badge, count: count_future);

    // The pending state misses the pattern, so the region renders empty.
    assert!(render.settle().is_pending());
    assert_eq!(render.html(&cx).as_deref(), Some("<nav></nav>"));

    // The badge appears when the count arrives.
    count.resolve(Ok(3));
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<nav><span>3</span></nav>")
    );
}

#[component]
async fn list_item(label: String) -> Result {
    view! { <li>(label)</li> }
}

#[component]
async fn list() -> Result {
    let labels = ["espresso", "grinder"];

    view! {
        <ul>
            for label in labels {
                list_item(key: label, label: label.to_string())
            }
        </ul>
    }
}

#[test]
fn a_loop_over_components_renders_every_iteration() {
    let cx = Cx::default();
    let mut render = render!(list);

    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<ul><li>espresso</li><li>grinder</li></ul>"),
    );
}
