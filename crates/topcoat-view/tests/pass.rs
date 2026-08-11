//! Scenario tests for the streaming pass runtime.
//!
//! Components here are the hand written form of what `view!` and
//! `#[component]` will generate: an async fn that mounts, runs its body
//! once, then loops over renders separated by the pass boundary. The driver
//! is polled manually with a noop waker, and I/O is stood in for by manually
//! fired triggers, so every scenario is deterministic.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
};

use topcoat_core::{
    context::{Cx, CxTestBuilder},
    error::{Error, Result},
};
use topcoat_view::{
    identity::SiteKey,
    pass::{
        Children, Deferred, Driver, PassReport, RenderBuffer, ViewToken, defer, defer_keyed, mount,
        pass_boundary,
    },
};

/// The call site of the enclosing expression, the way macro output derives
/// invocation identity.
macro_rules! site {
    () => {
        SiteKey::new(file!(), line!(), column!(), 0)
    };
}

fn cx() -> Cx {
    CxTestBuilder::new().build()
}

fn trigger() -> (Trigger, Fire) {
    let fired = Arc::new(AtomicBool::new(false));
    (Trigger(fired.clone()), Fire(fired))
}

struct Trigger(Arc<AtomicBool>);

impl Future for Trigger {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _ctx: &mut Context<'_>) -> Poll<()> {
        if self.0.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[derive(Clone)]
struct Fire(Arc<AtomicBool>);

impl Fire {
    fn fire(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Default)]
struct Counter(Arc<AtomicU32>);

impl Counter {
    fn bump(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn get(&self) -> u32 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn push(&self, event: impl Into<String>) {
        self.0.lock().unwrap().push(event.into());
    }

    fn contains(&self, event: &str) -> bool {
        self.0.lock().unwrap().iter().any(|e| e == event)
    }
}

struct DropWitness {
    log: Log,
    name: &'static str,
}

impl Drop for DropWitness {
    fn drop(&mut self) {
        self.log.push(format!("drop:{}", self.name));
    }
}

fn poll_driver(driver: &mut Driver<'_>) -> Poll<Option<PassReport>> {
    let mut ctx = Context::from_waker(Waker::noop());
    driver.poll_next_pass(&mut ctx)
}

#[track_caller]
fn sealed_pass(driver: &mut Driver<'_>) -> PassReport {
    match poll_driver(driver) {
        Poll::Ready(Some(report)) => report,
        other => panic!("expected a sealed pass, got {other:?}"),
    }
}

#[track_caller]
fn stream_ends(driver: &mut Driver<'_>) {
    assert!(
        matches!(poll_driver(driver), Poll::Ready(None)),
        "expected the stream to end"
    );
}

fn boom(message: &str) -> Error {
    anyhow::anyhow!("{message}").into()
}

fn block_on_noop<F: Future>(fut: F) -> F::Output {
    let mut fut = std::pin::pin!(fut);
    let mut ctx = Context::from_waker(Waker::noop());
    for _ in 0..1000 {
        if let Poll::Ready(output) = fut.as_mut().poll(&mut ctx) {
            return output;
        }
    }
    panic!("the future did not complete without external wakes");
}

// A component with no children and no defer: static content, one pass.
async fn static_page(bodies: Counter) -> Result<()> {
    let mount = mount();
    bodies.bump();
    let title = String::from("hello");
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<h1>");
        out.text(&title);
        out.markup("</h1>");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn static_page_seals_in_one_pass() {
    let bodies = Counter::default();
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), static_page(bodies.clone()));
    let p1 = sealed_pass(&mut driver);
    assert_eq!(p1.pass, 1);
    assert_eq!(p1.html, "<h1>hello</h1>");
    assert_eq!(p1.polls, 1);
    assert!(p1.page_error.is_none());
    assert_eq!(bodies.get(), 1);
    assert_eq!(driver.outstanding_deferred(), 0);
    stream_ends(&mut driver);
}

#[test]
fn interpolated_text_is_escaped() {
    async fn page() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.text("a<b & \"c\"");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), page());
    assert_eq!(sealed_pass(&mut driver).html, "a&lt;b &amp; \"c\"");
}

// The design doc's menu: a deferred list, skeleton first, keyed cards whose
// props borrow the deferred output through `Ready(&T)`.
async fn menu(cx: Cx, bodies: Counter, cards: Counter, io: Trigger) -> Result<()> {
    let mount = mount();
    bodies.bump();
    let mut io = Some(io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<h1>The menu</h1>");
        let drinks: Deferred<'_, Vec<String>> = defer(&cx, |_cx| {
            let io = io.take().expect("the load runs once");
            async move {
                io.await;
                vec![String::from("mojito"), String::from("mai-tai")]
            }
        });
        match drinks {
            Deferred::Pending => out.markup("<div class=skeleton></div>"),
            Deferred::Ready(list) => {
                out.markup("<ul>");
                for name in list {
                    children.advance_keyed(&mut out, site!(), name.as_str(), || {
                        card(cards.clone(), name)
                    })?;
                }
                out.markup("</ul>");
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn card(cards: Counter, name: &str) -> Result<()> {
    let mount = mount();
    cards.bump();
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<li>");
        out.text(&name.to_uppercase());
        out.markup("</li>");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn deferred_load_streams_in_two_passes_and_bodies_run_once() {
    let cx = cx();
    let (io, fire) = trigger();
    let bodies = Counter::default();
    let cards = Counter::default();
    let mut driver = Driver::new(
        cx.detach(),
        menu(cx.detach(), bodies.clone(), cards.clone(), io),
    );

    let p1 = sealed_pass(&mut driver);
    assert!(p1.html.contains("skeleton"));
    assert!(p1.html.contains("<h1>The menu</h1>"));
    assert_eq!(driver.outstanding_deferred(), 1);

    assert!(
        matches!(poll_driver(&mut driver), Poll::Pending),
        "waits for the load"
    );
    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.pass, 2);
    assert!(p2.html.contains("<li>MOJITO</li>"));
    assert!(p2.html.contains("<li>MAI-TAI</li>"));
    assert!(!p2.html.contains("skeleton"));

    // The headline claim: the body ran once across two passes, with no
    // memoization anywhere.
    assert_eq!(bodies.get(), 1);
    assert_eq!(cards.get(), 2);
    assert_eq!(p2.changed.len(), 3, "the menu and both new cards changed");
    stream_ends(&mut driver);
}

// A sequential chain: the inner component only becomes reachable once the
// outer data arrived, and registers its own defer at that point.
async fn outer_chain(cx: Cx, io1: Trigger, io2: Trigger) -> Result<()> {
    let mount = mount();
    let mut io1 = Some(io1);
    let mut io2 = Some(io2);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let first: Deferred<'_, String> = defer(&cx, |_cx| {
            let io = io1.take().expect("the load runs once");
            async move {
                io.await;
                String::from("first")
            }
        });
        match first {
            Deferred::Pending => out.markup("[outer skeleton]"),
            Deferred::Ready(value) => {
                out.text(value);
                let io = &mut io2;
                children.advance(&mut out, site!(), || {
                    inner_chain(cx.detach(), io.take().expect("born once"))
                })?;
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn inner_chain(cx: Cx, io: Trigger) -> Result<()> {
    let mount = mount();
    let mut io = Some(io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let second: Deferred<'_, String> = defer(&cx, |_cx| {
            let io = io.take().expect("the load runs once");
            async move {
                io.await;
                String::from("second")
            }
        });
        match second {
            Deferred::Pending => out.markup("[inner skeleton]"),
            Deferred::Ready(value) => out.text(value),
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn sequential_chain_peels_one_layer_per_pass() {
    let cx = cx();
    let (io1, fire1) = trigger();
    let (io2, fire2) = trigger();
    let mut driver = Driver::new(cx.detach(), outer_chain(cx.detach(), io1, io2));

    assert_eq!(sealed_pass(&mut driver).html, "[outer skeleton]");
    fire1.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.html, "first[inner skeleton]");
    assert_eq!(driver.outstanding_deferred(), 1);
    fire2.fire();
    let p3 = sealed_pass(&mut driver);
    assert_eq!(p3.pass, 3);
    assert_eq!(p3.html, "firstsecond");
    stream_ends(&mut driver);
}

// The straw man: a child prop borrowing the parent's body local across
// passes.
async fn lending_parent(cx: Cx, child_bodies: Counter, io: Trigger) -> Result<()> {
    let mount = mount();
    let title = String::from("borrowed");
    let mut io = Some(io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let bump: Deferred<'_, ()> = defer(&cx, |_cx| io.take().expect("the load runs once"));
        match bump {
            Deferred::Pending => out.markup("[p1]"),
            Deferred::Ready(()) => out.markup("[p2]"),
        }
        children.advance(&mut out, site!(), || {
            borrowing_child(child_bodies.clone(), &title)
        })?;
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn borrowing_child(bodies: Counter, s: &str) -> Result<()> {
    let mount = mount();
    bodies.bump();
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<b>");
        out.text(s);
        out.markup("</b>");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn child_prop_borrows_parent_body_local_across_passes() {
    let cx = cx();
    let (io, fire) = trigger();
    let child_bodies = Counter::default();
    let mut driver = Driver::new(
        cx.detach(),
        lending_parent(cx.detach(), child_bodies.clone(), io),
    );

    assert_eq!(sealed_pass(&mut driver).html, "[p1]<b>borrowed</b>");
    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.html, "[p2]<b>borrowed</b>");
    assert_eq!(child_bodies.get(), 1);
}

// A layout: catches its slot's error, keeps its other children alive, and
// renders error UI in place of the slot.
async fn layout(cx: Cx, log: Log, io: Trigger) -> Result<()> {
    let mount = mount();
    let mut io = Some(io);
    let mut failed: Option<Error> = None;
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<nav>");
        children.advance(&mut out, site!(), nav_widget)?;
        out.markup("</nav>");
        if let Some(error) = &failed {
            out.markup("<h1>error: ");
            out.text(&error.to_string());
            out.markup("</h1>");
        } else {
            let io = &mut io;
            let slot = children.advance_catching(&mut out, site!(), || {
                grid(cx.detach(), log.clone(), io.take().expect("born once"))
            });
            if let Err(error) = slot {
                out.markup("<h1>error: ");
                out.text(&error.to_string());
                out.markup("</h1>");
                failed = Some(error);
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn nav_widget() -> Result<()> {
    let mount = mount();
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("menu");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

// Defers a Result and propagates the deferred error with `?` on a later
// pass, completing the component future.
async fn grid(cx: Cx, log: Log, io: Trigger) -> Result<()> {
    let mount = mount();
    let _witness = DropWitness {
        log: log.clone(),
        name: "grid",
    };
    let mut io = Some(io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let data: Deferred<'_, Result<Vec<String>, String>> = defer(&cx, |_cx| {
            let io = io.take().expect("the load runs once");
            async move {
                io.await;
                Err(String::from("db down"))
            }
        });
        match data {
            Deferred::Pending => out.markup("<div class=skeleton></div>"),
            Deferred::Ready(result) => {
                let list = result.as_ref().map_err(|e| boom(e))?;
                for item in list {
                    out.text(item);
                }
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn deferred_error_on_a_later_pass_is_caught_by_the_layout() {
    let cx = cx();
    let (io, fire) = trigger();
    let log = Log::default();
    let mut driver = Driver::new(cx.detach(), layout(cx.detach(), log.clone(), io));

    let p1 = sealed_pass(&mut driver);
    assert!(p1.html.contains("skeleton"));
    assert!(p1.html.contains("<nav>menu</nav>"));

    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.pass, 2);
    assert!(p2.html.contains("<h1>error: db down</h1>"));
    assert!(
        p2.html.contains("<nav>menu</nav>"),
        "siblings outside the slot survive"
    );
    assert!(p2.page_error.is_none());
    assert!(log.contains("drop:grid"), "the errored subtree was dropped");
}

// The error handled at the source: matching `Ready` on the deferred Result
// instead of propagating. Nothing unwinds.
#[test]
fn deferred_error_can_be_handled_at_the_source() {
    async fn local_handler(cx: Cx, io: Trigger) -> Result<()> {
        let mount = mount();
        let mut io = Some(io);
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let data: Deferred<'_, Result<String, String>> = defer(&cx, |_cx| {
                let io = io.take().expect("the load runs once");
                async move {
                    io.await;
                    Err(String::from("unavailable"))
                }
            });
            match data {
                Deferred::Pending => out.markup("[skeleton]"),
                Deferred::Ready(Ok(value)) => out.text(value),
                Deferred::Ready(Err(error)) => {
                    out.markup("<p>could not load: ");
                    out.text(error);
                    out.markup("</p>");
                }
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.detach(), local_handler(cx.detach(), io));
    sealed_pass(&mut driver);
    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert!(p2.html.contains("<p>could not load: unavailable</p>"));
    assert!(p2.page_error.is_none());
}

// A `Drop` impl that reads a borrowed prop, at eviction and at end of
// request: the frame's compiler managed drop order makes both sound.
struct Audit<'a> {
    log: Log,
    label: &'static str,
    s: &'a str,
}

impl Drop for Audit<'_> {
    fn drop(&mut self) {
        self.log.push(format!("audit:{}:{}", self.label, self.s));
    }
}

async fn audited(log: Log, label: &'static str, s: &str) -> Result<()> {
    let mount = mount();
    let audit = Audit { log, label, s };
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("[");
        out.text(audit.label);
        out.markup(":");
        out.text(audit.s);
        out.markup("]");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn audit_parent(cx: Cx, log: Log, io: Trigger) -> Result<()> {
    let mount = mount();
    let title = String::from("The menu");
    let mut io = Some(io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let switch: Deferred<'_, ()> = defer(&cx, |_cx| io.take().expect("the load runs once"));
        match switch {
            Deferred::Pending => {
                children.advance(&mut out, site!(), || {
                    audited(log.clone(), "evictee", &title)
                })?;
            }
            Deferred::Ready(()) => {
                children.advance(&mut out, site!(), || audited(log.clone(), "keeper", &title))?;
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn arm_switch_evicts_the_orphan_and_drop_reads_the_borrowed_prop() {
    let cx = cx();
    let (io, fire) = trigger();
    let log = Log::default();
    let mut driver = Driver::new(cx.detach(), audit_parent(cx.detach(), log.clone(), io));

    assert_eq!(sealed_pass(&mut driver).html, "[evictee:The menu]");
    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.html, "[keeper:The menu]");
    assert!(
        log.contains("audit:evictee:The menu"),
        "eviction dropped the orphan"
    );

    // End of request: dropping the driver drops the tree in compiler managed
    // order, children before the locals they borrow.
    drop(driver);
    assert!(log.contains("audit:keeper:The menu"));
}

// An error with no catcher above it becomes the whole page error.
#[test]
fn uncaught_error_becomes_a_page_error() {
    // A body can fail before its first await; the component contract stays
    // async.
    #[expect(clippy::unused_async)]
    async fn fatal_root() -> Result<()> {
        let _mount = mount();
        Err(boom("fatal"))
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), fatal_root());
    let p1 = sealed_pass(&mut driver);
    let error = p1.page_error.expect("the root error surfaces");
    assert_eq!(error.to_string(), "fatal");
    stream_ends(&mut driver);
}

// Retention: keyed children are advanced, not reborn, and a settled pass is
// a single poll producing no changes.
async fn retained_menu(cx: Cx, cards: Counter, io1: Trigger, io2: Trigger) -> Result<()> {
    let mount = mount();
    let mut io1 = Some(io1);
    let mut io2 = Some(io2);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let _bump: Deferred<'_, ()> = defer(&cx, |_cx| io2.take().expect("the load runs once"));
        let drinks: Deferred<'_, Vec<String>> = defer(&cx, |_cx| {
            let io = io1.take().expect("the load runs once");
            async move {
                io.await;
                vec![String::from("mojito"), String::from("mai-tai")]
            }
        });
        match drinks {
            Deferred::Pending => out.markup("[skeleton]"),
            Deferred::Ready(list) => {
                for name in list {
                    children.advance_keyed(&mut out, site!(), name.as_str(), || {
                        card(cards.clone(), name)
                    })?;
                }
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn keyed_children_are_retained_and_settled_passes_are_one_unchanged_poll() {
    let cx = cx();
    let (io1, fire1) = trigger();
    let (io2, fire2) = trigger();
    let cards = Counter::default();
    let mut driver = Driver::new(
        cx.detach(),
        retained_menu(cx.detach(), cards.clone(), io1, io2),
    );

    sealed_pass(&mut driver);
    fire1.fire();
    let p2 = sealed_pass(&mut driver);
    assert!(p2.html.contains("MOJITO"));

    fire2.fire();
    let p3 = sealed_pass(&mut driver);
    assert_eq!(p3.pass, 3);
    assert_eq!(cards.get(), 2, "the cards were advanced, not reborn");
    assert_eq!(p3.polls, 1, "a settled tree is a single poll per pass");
    assert!(p3.changed.is_empty(), "identical renders change nothing");
    stream_ends(&mut driver);
}

// A component born on a later pass whose body parks on I/O: the pass cannot
// seal until the birth settles, and a deferred completion arriving mid pass
// waits for the next snapshot.
async fn slow_birth_page(cx: Cx, io1: Trigger, birth_io: Trigger, extra_io: Trigger) -> Result<()> {
    let mount = mount();
    let mut io1 = Some(io1);
    let mut birth_io = Some(birth_io);
    let mut extra_io = Some(extra_io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let extra: Deferred<'_, ()> =
            defer(&cx, |_cx| extra_io.take().expect("the load runs once"));
        out.markup(match extra {
            Deferred::Pending => "[extra?]",
            Deferred::Ready(()) => "[extra!]",
        });
        let first: Deferred<'_, ()> = defer(&cx, |_cx| io1.take().expect("the load runs once"));
        match first {
            Deferred::Pending => out.markup("[skeleton]"),
            Deferred::Ready(()) => {
                let io = &mut birth_io;
                children.advance(&mut out, site!(), || {
                    slow_child(io.take().expect("born once"))
                })?;
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn slow_child(io: Trigger) -> Result<()> {
    let mount = mount();
    io.await;
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("done");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn parked_births_block_the_seal_and_mid_pass_completions_wait_for_the_snapshot() {
    let cx = cx();
    let (io1, fire1) = trigger();
    let (birth_io, fire_birth) = trigger();
    let (extra_io, fire_extra) = trigger();
    let mut driver = Driver::new(
        cx.detach(),
        slow_birth_page(cx.detach(), io1, birth_io, extra_io),
    );

    let p1 = sealed_pass(&mut driver);
    assert_eq!(p1.html, "[extra?][skeleton]");

    fire1.fire();
    assert!(
        matches!(poll_driver(&mut driver), Poll::Pending),
        "the birth parks the pass"
    );
    // The extra load completes while pass 2 is mid flight: its result must
    // wait for the next snapshot.
    fire_extra.fire();
    fire_birth.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.pass, 2);
    assert_eq!(
        p2.html, "[extra?]done",
        "mid pass completions stay invisible"
    );

    let p3 = sealed_pass(&mut driver);
    assert_eq!(p3.html, "[extra!]done");
    stream_ends(&mut driver);
}

// A late birth that fails while the tree is suspended: the error unwinds the
// intermediate page, is stashed at the catching layout, and an automatic
// extra pass renders the fallback.
async fn stash_layout(cx: Cx, log: Log, io1: Trigger, inner_io: Trigger) -> Result<()> {
    let mount = mount();
    let mut props = Some((io1, inner_io));
    let mut failed: Option<Error> = None;
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        if let Some(error) = &failed {
            out.markup("<h1>error: ");
            out.text(&error.to_string());
            out.markup("</h1>");
        } else {
            let props = &mut props;
            let slot = children.advance_catching(&mut out, site!(), || {
                let (io1, inner_io) = props.take().expect("born once");
                stash_page(cx.detach(), log.clone(), io1, inner_io)
            });
            if let Err(error) = slot {
                out.markup("<h1>error: ");
                out.text(&error.to_string());
                out.markup("</h1>");
                failed = Some(error);
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn stash_page(cx: Cx, log: Log, io1: Trigger, inner_io: Trigger) -> Result<()> {
    let mount = mount();
    let _witness = DropWitness {
        log: log.clone(),
        name: "page",
    };
    let mut io1 = Some(io1);
    let mut inner_io = Some(inner_io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        let first: Deferred<'_, ()> = defer(&cx, |_cx| io1.take().expect("the load runs once"));
        match first {
            Deferred::Pending => out.markup("[page skeleton]"),
            Deferred::Ready(()) => {
                children.advance(&mut out, site!(), || sibling(log.clone()))?;
                let io = &mut inner_io;
                children.advance(&mut out, site!(), || {
                    failing_inner(log.clone(), io.take().expect("born once"))
                })?;
            }
        }
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn sibling(log: Log) -> Result<()> {
    let mount = mount();
    let _witness = DropWitness {
        log,
        name: "sibling",
    };
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("[sib]");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn failing_inner(log: Log, io: Trigger) -> Result<()> {
    let _mount = mount();
    let _witness = DropWitness { log, name: "inner" };
    io.await;
    Err(boom("late boom"))
}

#[test]
fn mid_suspension_error_is_stashed_and_rendered_on_an_automatic_extra_pass() {
    let cx = cx();
    let (io1, fire1) = trigger();
    let (inner_io, fire_inner) = trigger();
    let log = Log::default();
    let mut driver = Driver::new(
        cx.detach(),
        stash_layout(cx.detach(), log.clone(), io1, inner_io),
    );

    let p1 = sealed_pass(&mut driver);
    assert!(p1.html.contains("[page skeleton]"));

    fire1.fire();
    assert!(
        matches!(poll_driver(&mut driver), Poll::Pending),
        "the birth parks pass 2"
    );
    fire_inner.fire();
    let p3 = sealed_pass(&mut driver);
    assert_eq!(
        p3.pass, 3,
        "the caught error rolls into an automatic extra pass"
    );
    assert!(p3.html.contains("<h1>error: late boom</h1>"));
    // The whole slot subtree died: the failing component, its healthy
    // sibling, and the intermediate page.
    assert!(log.contains("drop:inner"));
    assert!(log.contains("drop:page"));
    assert!(log.contains("drop:sibling"));
}

// Deferred loads registered in a loop, told apart by key.
#[test]
fn keyed_defers_in_a_loop_resolve_independently() {
    async fn keyed(cx: Cx) -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            for i in 0..2u32 {
                match defer_keyed(&cx, i, move |_cx| async move { i * 10 }) {
                    Deferred::Pending => out.markup("?"),
                    Deferred::Ready(value) => out.text(&value.to_string()),
                }
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let mut driver = Driver::new(cx.detach(), keyed(cx.detach()));
    assert_eq!(sealed_pass(&mut driver).html, "??");
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.html, "010");
    stream_ends(&mut driver);
}

// An unkeyed invocation that repeats hits the identity diagnostic.
#[test]
#[should_panic(expected = "advanced twice")]
fn unkeyed_repeated_invocation_panics() {
    async fn repeats() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            for _ in 0..2 {
                children.advance(&mut out, SiteKey::new("same", 1, 1, 0), nav_widget)?;
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), repeats());
    sealed_pass(&mut driver);
}

// Blocking mode: run every pass without streaming and return the final
// document. Loads that resolve without external I/O complete inline.
#[test]
fn render_blocking_returns_only_the_final_document() {
    async fn instant_menu(cx: Cx) -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            match defer(&cx, |_cx| async move { String::from("ready") }) {
                Deferred::Pending => out.markup("[skeleton]"),
                Deferred::Ready(value) => out.text(value),
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let mut driver = Driver::new(cx.detach(), instant_menu(cx.detach()));
    let html = block_on_noop(driver.render_blocking()).expect("renders");
    assert_eq!(html, "ready");
}

// The task model: the driver and every component future are Send.
#[test]
fn the_driver_and_component_futures_are_send() {
    fn assert_send<T: Send>(_: &T) {}
    let cx = cx();
    let (io, _fire) = trigger();
    let fut = menu(cx.detach(), Counter::default(), Counter::default(), io);
    assert_send(&fut);
    let driver = Driver::new(cx.detach(), fut);
    assert_send(&driver);
}

// Child content: the trailing block compiles to an anonymous component owned
// by its creator. The receiver holds only a token and places it.
async fn content_home(cx: Cx, content_bodies: Counter, expand_io: Trigger) -> Result<()> {
    let mount = mount();
    let user = String::from("carl");
    let mut children = Children::new();
    let mut expand_io = Some(expand_io);
    loop {
        let mut out = RenderBuffer::new();
        // The trailing block of `panel(..) { <p>(user)</p> }`.
        let token = children.content(site!(), || content_block(content_bodies.clone(), &user))?;
        let io = &mut expand_io;
        children.advance(&mut out, site!(), || {
            panel(cx.detach(), token, io.take().expect("born once"))
        })?;
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn content_block(bodies: Counter, user: &str) -> Result<()> {
    let mount = mount();
    bodies.bump();
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<p>");
        out.text(user);
        out.markup("</p>");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

async fn panel(cx: Cx, token: ViewToken, expand_io: Trigger) -> Result<()> {
    let mount = mount();
    let mut expand_io = Some(expand_io);
    let mut children = Children::new();
    loop {
        let mut out = RenderBuffer::new();
        out.markup("<section>");
        match defer(&cx, |_cx| expand_io.take().expect("the load runs once")) {
            Deferred::Pending => out.markup("[collapsed]"),
            Deferred::Ready(()) => out.place(token)?,
        }
        out.markup("</section>");
        children.sweep();
        mount.finish_render(out);
        pass_boundary(&mount, &mut children).await?;
    }
}

#[test]
fn content_runs_unplaced_and_placement_shows_its_current_state() {
    let cx = cx();
    let (expand_io, fire) = trigger();
    let content_bodies = Counter::default();
    let mut driver = Driver::new(
        cx.detach(),
        content_home(cx.detach(), content_bodies.clone(), expand_io),
    );

    let p1 = sealed_pass(&mut driver);
    assert_eq!(p1.html, "<section>[collapsed]</section>");
    assert_eq!(content_bodies.get(), 1, "unplaced content still runs");

    fire.fire();
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.html, "<section><p>carl</p></section>");
    assert_eq!(
        content_bodies.get(),
        1,
        "placement shows the warm content, no rebirth"
    );
}

// A content failure is delivered at placement, where the placer catches it
// like a layout catches its slot.
#[test]
fn placement_delivers_the_content_error_to_the_placer() {
    async fn failing_content() -> Result<()> {
        let _mount = mount();
        Err(boom("content boom"))
    }
    async fn catching_panel(token: ViewToken) -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            if let Err(error) = out.place(token) {
                out.markup("[caught: ");
                out.text(&error.to_string());
                out.markup("]");
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn creator() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let token = children.content(site!(), failing_content)?;
            children.advance(&mut out, site!(), || catching_panel(token))?;
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let mut driver = Driver::new(cx.detach(), creator());
    let p1 = sealed_pass(&mut driver);
    assert_eq!(p1.html, "[caught: content boom]");
    assert!(p1.page_error.is_none(), "nothing unwinds");
}

// Content that fails and is never placed hands the error back to its owner
// on an automatic extra pass.
#[test]
fn unplaced_content_error_falls_back_to_the_owner() {
    async fn failing_content() -> Result<()> {
        let _mount = mount();
        Err(boom("content boom"))
    }
    async fn never_places() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.markup("[collapsed]");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn creator() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let token = children.content(site!(), failing_content)?;
            let _ = token;
            children.advance(&mut out, site!(), never_places)?;
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let mut driver = Driver::new(cx.detach(), creator());
    let p2 = sealed_pass(&mut driver);
    assert_eq!(p2.pass, 2, "custody rolls into an automatic extra pass");
    let error = p2
        .page_error
        .expect("the owner propagated the content error");
    assert_eq!(error.to_string(), "content boom");
}

// One slot cannot fill two positions.
#[test]
#[should_panic(expected = "placed twice")]
fn double_placement_panics() {
    async fn healthy_content() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.markup("x");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn creator() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let token = children.content(site!(), healthy_content)?;
            out.place(token)?;
            out.place(token)?;
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), creator());
    sealed_pass(&mut driver);
}

// Content contains its own streaming component: it defers, skeletons, and
// resolves inside the placed region across passes.
#[test]
fn content_streams_its_own_deferred_data() {
    async fn streaming_content(cx: Cx, io: Trigger) -> Result<()> {
        let mount = mount();
        let mut io = Some(io);
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            match defer(&cx, |_cx| {
                let io = io.take().expect("the load runs once");
                async move {
                    io.await;
                    String::from("loaded")
                }
            }) {
                Deferred::Pending => out.markup("[content skeleton]"),
                Deferred::Ready(value) => out.text(value),
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn creator(cx: Cx, io: Trigger) -> Result<()> {
        let mount = mount();
        let mut io = Some(io);
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let content_io = &mut io;
            let token = children.content(site!(), || {
                streaming_content(cx.detach(), content_io.take().expect("born once"))
            })?;
            out.markup("<aside>");
            out.place(token)?;
            out.markup("</aside>");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.detach(), creator(cx.detach(), io));
    assert_eq!(
        sealed_pass(&mut driver).html,
        "<aside>[content skeleton]</aside>"
    );
    fire.fire();
    assert_eq!(sealed_pass(&mut driver).html, "<aside>loaded</aside>");
    stream_ends(&mut driver);
}

// A stale token, held after the creator swept the content away, places to
// nothing instead of failing.
#[test]
fn placing_an_evicted_token_renders_empty() {
    async fn short_content() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.markup("gone soon");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn late_placer(token: ViewToken) -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.markup("<section>");
            out.place(token)?;
            out.markup("</section>");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    async fn creator(cx: Cx, io: Trigger) -> Result<()> {
        let mount = mount();
        let mut io = Some(io);
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            let stop: Deferred<'_, ()> = defer(&cx, |_cx| io.take().expect("the load runs once"));
            let mut token = None;
            if matches!(stop, Deferred::Pending) {
                token = Some(children.content(site!(), short_content)?);
            }
            if let Some(token) = token {
                let placer = token;
                children.advance(&mut out, site!(), move || late_placer(placer))?;
            }
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }

    let cx = cx();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.detach(), creator(cx.detach(), io));
    assert_eq!(
        sealed_pass(&mut driver).html,
        "<section>gone soon</section>"
    );
    fire.fire();
    // The creator swept both the content and the placer; the tree is empty.
    assert_eq!(sealed_pass(&mut driver).html, "");
    stream_ends(&mut driver);
}

// The render positions accept the same value types the view macros support,
// through the instruction buffer backing.
#[test]
fn render_positions_accept_the_view_value_types() {
    async fn typed() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.markup_static(&"<p data-n=\"");
            out.attribute_value(42u32);
            out.markup_static(&"\">");
            out.node(7i64);
            out.node(" & ");
            out.node(true);
            out.markup_static(&"</p>");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), typed());
    assert_eq!(
        sealed_pass(&mut driver).html,
        "<p data-n=\"42\">7 &amp; true</p>"
    );
}

// Response metadata declared in a render is recorded and reported.
#[test]
fn renders_declare_response_status_and_headers() {
    async fn page() -> Result<()> {
        let mount = mount();
        let mut children = Children::new();
        loop {
            let mut out = RenderBuffer::new();
            out.node(http::StatusCode::NOT_FOUND);
            out.markup("nothing here");
            children.sweep();
            mount.finish_render(out);
            pass_boundary(&mount, &mut children).await?;
        }
    }
    let cx = cx();
    let mut driver = Driver::new(cx.detach(), page());
    let p1 = sealed_pass(&mut driver);
    assert_eq!(p1.html, "nothing here");
    assert_eq!(p1.status_code, Some(http::StatusCode::NOT_FOUND));
}
