//! Integration tests for the live render runtime.
//!
//! The component functions here are hand-written versions of the code the
//! `view!` and `#[component]` macros will generate: body locals in the outer
//! scope, the frame's `RefreshSet` owned by an inner block, fused children
//! invoked into it, reactive nodes pushed as futures, and each run of a
//! node's arms opening a frame of its own. The generated form is a plain
//! `fn` returning `impl Future + Send + 'frame`; these are written as
//! `async fn` for brevity, and the driver's `Send` bound checks the same
//! property. The driver polls the render as one task and reads the document
//! chunk by chunk between polls.

use std::{
    fmt,
    future::{Future, poll_fn},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{
    internal::{block, reserve},
    live::{
        Deferred, Fill, RefreshSet, defer, run_node,
        test_support::{Channel, TestRender, channel},
    },
};

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

/// Work that never completes and records when it is dropped.
struct Hung {
    dropped: Arc<AtomicBool>,
}

impl Future for Hung {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, _task: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }
}

impl Drop for Hung {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

/// A future that must stay lazy: its first poll is the failure.
fn never_polled() -> impl Future<Output = Result<Vec<String>>> + Send {
    poll_fn(|_task| -> Poll<Result<Vec<String>>> { panic!("a lazy future was polled") })
}

/// A render that must stay parked: its first poll is the failure.
fn never_rendered(_fill: Fill) -> impl Future<Output = Result<()>> + Send {
    poll_fn(|_task| -> Poll<Result<()>> { panic!("an unconsumed handle's render ran") })
}

/// The error a failing load in these tests reports.
#[derive(Debug)]
struct BackendDown;

impl fmt::Display for BackendDown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the orders backend is down")
    }
}

impl std::error::Error for BackendDown {}

/// A fused child component with props borrowing the caller's locals.
async fn avatar(cx: &Cx, name: &str, fill: Fill) -> Result<()> {
    let refresh = RefreshSet::new();
    let view = block(cx, |b| {
        b.markup(&"<span>");
        b.node(name);
        b.markup(&"</span>");
    });
    fill.fill(view);
    refresh.run().await
}

/// A component with a body local, a fused child borrowing it, and one
/// reactive node over a deferred load.
async fn profile(cx: &Cx, orders: Manual<Result<Vec<String>>>, fill: Fill) -> Result<()> {
    // The body: its locals live in the outer scope.
    let user = String::from("Ada");

    // The tail view expansion: an inner block owning the frame's set.
    {
        let mut refresh = RefreshSet::new();

        let child0 = refresh.invoke(|f| avatar(cx, &user, f));

        let (node0, node0_slot) = reserve();
        let node0_ticket = refresh.ticket();
        let reactive0 = defer(cx, orders);
        refresh.push(run_node(reactive0, move |state| {
            // Per-call copies moved into the arm future.
            let slot = node0_slot;
            let ticket = node0_ticket;
            let cx = cx;
            async move {
                // One run of the arms is a frame of its own.
                let mut arm = RefreshSet::new();
                let view = match state {
                    Deferred::Pending => block(cx, |b| b.markup(&"<p>orders loading</p>")),
                    Deferred::Ready(orders) => {
                        // The arm's `?` is real: it fails the node and
                        // climbs the frame tree.
                        let orders = orders?;
                        block(cx, |b| {
                            b.markup(&"<ul>");
                            for order in &orders {
                                b.markup(&"<li>");
                                b.node(order.as_str());
                                b.markup(&"</li>");
                            }
                            b.markup(&"</ul>");
                        })
                    }
                };
                arm.barrier().await?;
                slot.refill(view);
                ticket.hand_over();
                arm.run().await
            }
        }));

        // The join, then the burst, then the yield.
        refresh.barrier().await?;
        let view = block(cx, |b| {
            b.markup(&"<h1>");
            b.node(user.as_str());
            b.markup(&"</h1>");
            b.view(child0);
            b.view(node0);
        });
        fill.fill(view);
        refresh.run().await
    }
}

/// A component with deferred content of its own, adopted unfused by
/// [`shared_page`] so its swaps land wherever its handle is spliced.
async fn activity_feed(
    cx: &Cx,
    runs: &AtomicUsize,
    items: Manual<Result<String>>,
    fill: Fill,
) -> Result<()> {
    runs.fetch_add(1, Ordering::SeqCst);
    let mut refresh = RefreshSet::new();

    let (node0, node0_slot) = reserve();
    let node0_ticket = refresh.ticket();
    let reactive0 = defer(cx, items);
    refresh.push(run_node(reactive0, move |state| {
        let slot = node0_slot;
        let ticket = node0_ticket;
        let cx = cx;
        async move {
            let mut arm = RefreshSet::new();
            let view = match state {
                Deferred::Pending => block(cx, |b| b.markup(&"<em>feed loading</em>")),
                Deferred::Ready(items) => {
                    let items = items?;
                    block(cx, |b| {
                        b.markup(&"<em>");
                        b.node(items);
                        b.markup(&"</em>");
                    })
                }
            };
            arm.barrier().await?;
            slot.refill(view);
            ticket.hand_over();
            arm.run().await
        }
    }));

    refresh.barrier().await?;
    let view = block(cx, |b| {
        b.markup(&"<aside>");
        b.view(node0);
        b.markup(&"</aside>");
    });
    fill.fill(view);
    refresh.run().await
}

/// A page adopting a shared handle in its body and splicing clones of it
/// into every arm of a channel-driven node.
async fn shared_page(
    cx: &Cx,
    runs: &AtomicUsize,
    items: Manual<Result<String>>,
    states: Channel<&'static str>,
    fill: Fill,
) -> Result<()> {
    let mut refresh = RefreshSet::new();

    // `let feed = view! { activity_feed() };` in the guide: the handle
    // belongs to the body's frame, not to either arm.
    let feed = refresh.adopt(|f| activity_feed(cx, runs, items, f));

    let (node0, node0_slot) = reserve();
    let node0_ticket = refresh.ticket();
    let feed_for_arms = feed.clone();
    refresh.push(run_node(states, move |state| {
        let slot = node0_slot;
        let ticket = node0_ticket;
        let cx = cx;
        // Handles are captured by clone, one per arm run.
        let feed = feed_for_arms.clone();
        async move {
            let mut arm = RefreshSet::new();
            let feed_view = arm.splice(feed);
            let view = block(cx, |b| {
                b.markup(&"<div>");
                b.node(state);
                b.markup(&" ");
                b.view(feed_view);
                b.markup(&"</div>");
            });
            arm.barrier().await?;
            slot.refill(view);
            ticket.hand_over();
            arm.run().await
        }
    }));

    refresh.barrier().await?;
    let view = block(cx, |b| {
        b.markup(&"<main>");
        b.view(node0);
        b.markup(&"</main>");
    });
    fill.fill(view);
    refresh.run().await
}

/// A page whose first arm starts inner work that never finishes, so a new
/// state must cancel it.
async fn race_page(cx: &Cx, states: Channel<u32>, hung: Arc<AtomicBool>, fill: Fill) -> Result<()> {
    let mut refresh = RefreshSet::new();

    let (node0, node0_slot) = reserve();
    let node0_ticket = refresh.ticket();
    refresh.push(run_node(states, move |state| {
        let slot = node0_slot;
        let ticket = node0_ticket;
        let cx = cx;
        let hung = hung.clone();
        async move {
            let mut arm = RefreshSet::new();
            let view = if state == 1 {
                // Slow inner work owned by this arm's frame.
                arm.push(Hung { dropped: hung });
                block(cx, |b| b.markup(&"<p>one</p>"))
            } else {
                block(cx, |b| b.markup(&"<p>two</p>"))
            };
            arm.barrier().await?;
            slot.refill(view);
            ticket.hand_over();
            arm.run().await
        }
    }));

    refresh.barrier().await?;
    let view = block(cx, |b| b.view(node0));
    fill.fill(view);
    refresh.run().await
}

/// A page creating lazy work and never consuming it.
async fn lazy_page(cx: &Cx, fill: Fill) -> Result<()> {
    let mut refresh = RefreshSet::new();

    // A `Defer` dropped unconsumed never polls its future.
    let unconsumed = defer(cx, never_polled());

    // An adopted handle whose clones all drop unstarted never runs.
    let handle = refresh.adopt(never_rendered);
    let clone = handle.clone();
    drop(handle);
    drop(clone);
    drop(unconsumed);

    refresh.barrier().await?;
    let view = block(cx, |b| b.markup(&"<p>lazy</p>"));
    fill.fill(view);
    refresh.run().await
}

#[test]
fn first_paint_then_swap() {
    let cx = Cx::default();
    let (orders, orders_future) = manual::<Result<Vec<String>>>();
    let mut render = TestRender::new(|fill| profile(&cx, orders_future, fill));

    // The first chunk carries the shell, the fused child, and the skeleton.
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<h1>Ada</h1><span>Ada</span><p>orders loading</p>"),
    );

    // The deferred data arrives: the ready arm swaps in and the render
    // completes.
    orders.resolve(Ok(vec![
        "espresso machine".to_string(),
        "grinder".to_string(),
    ]));
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<h1>Ada</h1><span>Ada</span><ul><li>espresso machine</li><li>grinder</li></ul>"),
    );
}

#[test]
fn lazy_work_never_runs() {
    let cx = Cx::default();
    let mut render = TestRender::new(|fill| lazy_page(&cx, fill));

    // The panicking futures inside prove laziness by not firing.
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
    assert_eq!(render.html(&cx).as_deref(), Some("<p>lazy</p>"));
}

#[test]
fn shared_handle_renders_once_across_swaps() {
    let cx = Cx::default();
    let runs = AtomicUsize::new(0);
    let (items, items_future) = manual::<Result<String>>();
    let (states, state_feed) = channel::<&'static str>();

    states.send("A");
    let mut render =
        TestRender::new(|fill| shared_page(&cx, &runs, items_future, state_feed, fill));

    // The first arm's splice starts the one render of the feed.
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<main><div>A <aside><em>feed loading</em></aside></div></main>"),
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // A new state swaps the arm; the new arm's splice is served from the
    // cell's cache, and the feed's render is neither cancelled nor
    // restarted.
    states.send("B");
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<main><div>B <aside><em>feed loading</em></aside></div></main>"),
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // The feed's own swap lands inside the arm that replaced the one it
    // first appeared in.
    items.resolve(Ok(String::from("3 new reviews")));
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(
        render.html(&cx).as_deref(),
        Some("<main><div>B <aside><em>3 new reviews</em></aside></div></main>"),
    );
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // Retiring the channel lets the node, and with it the page, complete.
    states.close();
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
}

#[test]
fn arm_error_climbs_to_the_root() {
    let cx = Cx::default();
    let (orders, orders_future) = manual::<Result<Vec<String>>>();
    let mut render = TestRender::new(|fill| profile(&cx, orders_future, fill));

    // The first paint goes out before anything fails.
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert!(render.html(&cx).is_some());

    // The ready arm's `?` fails the node, and the error climbs the frame
    // tree to the root future's output with its type intact.
    orders.resolve(Err(BackendDown.into()));
    let Poll::Ready(Err(error)) = render.settle() else {
        panic!("the error did not climb to the root");
    };
    assert!(error.downcast_ref::<BackendDown>().is_some());
}

#[test]
fn a_new_state_cancels_the_shown_arms_work() {
    let cx = Cx::default();
    let (states, state_feed) = channel::<u32>();
    let dropped = Arc::new(AtomicBool::new(false));

    states.send(1);
    let mut render = TestRender::new(|fill| race_page(&cx, state_feed, dropped.clone(), fill));

    // The first arm renders while its inner work hangs.
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(render.html(&cx).as_deref(), Some("<p>one</p>"));
    assert!(!dropped.load(Ordering::SeqCst));

    // A new state drops the shown arm's frame, cancelling its work, and the
    // new arm renders.
    states.send(2);
    assert!(render.settle().is_pending());
    assert!(render.take_dirty());
    assert_eq!(render.html(&cx).as_deref(), Some("<p>two</p>"));
    assert!(dropped.load(Ordering::SeqCst));

    // Retiring the channel completes the node and the page.
    states.close();
    assert!(matches!(render.settle(), Poll::Ready(Ok(()))));
}

#[test]
fn the_root_future_is_send() {
    fn assert_send<T: Send>(_value: &T) {}

    let cx = Cx::default();
    let runs = AtomicUsize::new(0);
    let (_items, items_future) = manual::<Result<String>>();
    let (_states, state_feed) = channel::<&'static str>();

    // The render reaches shared state with no locks, yet stays `Send`: the
    // scope travels with the driver, not inside the future. `TestRender`
    // holds the root future and requires it `Send`.
    let render = TestRender::new(|fill| shared_page(&cx, &runs, items_future, state_feed, fill));
    assert_send(&render);
}
