//! Scenario tests for the component future model.
//!
//! Every component here is the hand desugared form of what `#[component]` and
//! `view!` would generate: an async fn that registers itself, runs its body
//! once, then loops over renders separated by [`pass_boundary`]. The tests
//! assert the design's claims: bodies run once, renders re-run per pass,
//! borrows of frame locals survive across passes, errors travel as future
//! completion, and eviction is a drop.

use streaming_model::*;

// A component with no children and no defer: static content.
async fn static_page(cx: Cx, me: CompId) -> Result {
    let mount = cx.register(me, "page");
    let title = String::from("hello");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        out.push_str("<h1>");
        interp(&mut out, || title.clone());
        out.push_str("</h1>");
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn static_page_seals_in_one_pass() {
    let cx = Cx::new();
    let mut driver = Driver::new(cx.clone(), Box::pin(static_page(cx.clone(), ROOT)));
    let p1 = driver.next_pass();
    assert_eq!(p1.pass, 1);
    assert_eq!(p1.html, "<h1>hello</h1>");
    assert_eq!(p1.polls, 1);
    assert_eq!(cx.outstanding_deferred(), 0);
    assert_eq!(cx.log_count("body:page"), 1);
    assert_eq!(cx.log_count("render:page"), 1);
}

// The doc's menu: a deferred list, skeleton first, cards on the second pass.
// Card props borrow the deferred output through `Ready(&T)`.
async fn menu(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "menu");
    let drinks: Defer<Vec<String>> = Defer::spawn(&cx, "drinks", async move {
        io.await;
        vec![String::from("mojito"), String::from("mai-tai")]
    });
    let title = String::from("The menu");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        out.push_str("<h1>");
        interp(&mut out, || title.clone());
        out.push_str("</h1>");
        match drinks.poll() {
            Deferred::Pending => out.push_str("<div class=skeleton></div>"),
            Deferred::Ready(list) => {
                out.push_str("<ul>");
                for name in list {
                    children.advance(&cx, &mut out, name, |id| {
                        Box::pin(card(cx.clone(), id, name))
                    })?;
                }
                out.push_str("</ul>");
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn card(cx: Cx, me: CompId, name: &str) -> Result {
    let mount = cx.register(me, "card");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        out.push_str("<li>");
        interp(&mut out, || name.to_uppercase());
        out.push_str("</li>");
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn deferred_load_streams_in_two_passes_and_bodies_run_once() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(menu(cx.clone(), ROOT, io)));

    let p1 = driver.next_pass();
    assert!(p1.html.contains("skeleton"));
    assert!(p1.html.contains("<h1>The menu</h1>"));
    assert_eq!(cx.outstanding_deferred(), 1);

    fire.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.pass, 2);
    assert!(p2.html.contains("<li>MOJITO</li>"));
    assert!(p2.html.contains("<li>MAI-TAI</li>"));
    assert!(!p2.html.contains("skeleton"));
    assert_eq!(cx.outstanding_deferred(), 0);

    // The headline claim: the body ran once even though the page rendered
    // twice, with no memoization anywhere.
    assert_eq!(cx.log_count("body:menu"), 1);
    assert_eq!(cx.log_count("render:menu"), 2);
    assert_eq!(cx.log_count("body:card"), 2);
    assert_eq!(cx.log_count("render:card"), 2);
}

// A sequential chain: the inner component only becomes reachable once the
// outer data arrived, and registers its own defer at that point.
async fn outer_chain(cx: Cx, me: CompId, io1: Trigger, io2: Trigger) -> Result {
    let mount = cx.register(me, "outer");
    let first: Defer<String> = Defer::spawn(&cx, "first", async move {
        io1.await;
        String::from("first")
    });
    let mut io2 = Some(io2);
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match first.poll() {
            Deferred::Pending => out.push_str("[outer skeleton]"),
            Deferred::Ready(value) => {
                interp(&mut out, || value.clone());
                let io = &mut io2;
                children.advance(&cx, &mut out, "inner", |id| {
                    Box::pin(inner_chain(cx.clone(), id, io.take().unwrap()))
                })?;
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn inner_chain(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "inner");
    let second: Defer<String> = Defer::spawn(&cx, "second", async move {
        io.await;
        String::from("second")
    });
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match second.poll() {
            Deferred::Pending => out.push_str("[inner skeleton]"),
            Deferred::Ready(value) => interp(&mut out, || value.clone()),
        }
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn sequential_chain_peels_one_layer_per_pass() {
    let cx = Cx::new();
    let (io1, fire1) = trigger();
    let (io2, fire2) = trigger();
    let mut driver = Driver::new(
        cx.clone(),
        Box::pin(outer_chain(cx.clone(), ROOT, io1, io2)),
    );

    let p1 = driver.next_pass();
    assert_eq!(p1.html, "[outer skeleton]");

    fire1.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.html, "first[inner skeleton]");
    assert_eq!(cx.outstanding_deferred(), 1);

    fire2.fire();
    let p3 = driver.next_pass();
    assert_eq!(p3.pass, 3);
    assert_eq!(p3.html, "firstsecond");
    assert_eq!(cx.log_count("body:outer"), 1);
    assert_eq!(cx.log_count("body:inner"), 1);
}

// Two independent defers observed by one component.
async fn two_defers(cx: Cx, me: CompId, io_a: Trigger, io_b: Trigger) -> Result {
    let mount = cx.register(me, "two");
    let a: Defer<String> = Defer::spawn(&cx, "a", async move {
        io_a.await;
        String::from("A")
    });
    let b: Defer<String> = Defer::spawn(&cx, "b", async move {
        io_b.await;
        String::from("B")
    });
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match a.poll() {
            Deferred::Pending => out.push_str("[a?]"),
            Deferred::Ready(v) => interp(&mut out, || format!("[{v}]")),
        }
        match b.poll() {
            Deferred::Pending => out.push_str("[b?]"),
            Deferred::Ready(v) => interp(&mut out, || format!("[{v}]")),
        }
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn defers_completing_together_batch_into_one_pass() {
    let cx = Cx::new();
    let (io_a, fire_a) = trigger();
    let (io_b, fire_b) = trigger();
    let mut driver = Driver::new(
        cx.clone(),
        Box::pin(two_defers(cx.clone(), ROOT, io_a, io_b)),
    );

    assert_eq!(driver.next_pass().html, "[a?][b?]");
    fire_a.fire();
    fire_b.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.html, "[A][B]");
    assert_eq!(cx.outstanding_deferred(), 0);
    assert_eq!(cx.log_count("render:two"), 2);
}

#[test]
fn completion_during_a_pass_waits_for_the_next_snapshot() {
    let cx = Cx::new();
    let (io_a, fire_a) = trigger();
    let (io_b, fire_b) = trigger();
    let mut driver = Driver::new(
        cx.clone(),
        Box::pin(two_defers(cx.clone(), ROOT, io_a, io_b)),
    );

    driver.next_pass();
    fire_a.fire();
    driver.begin_pass();
    assert!(driver.pump().is_none());
    // B completes while pass 2 is mid flight: the snapshot must not move.
    fire_b.fire();
    let p2 = driver.try_seal().expect("pass 2 seals");
    assert_eq!(p2.html, "[A][b?]");
    let p3 = driver.next_pass();
    assert_eq!(p3.html, "[A][B]");
}

// The straw man from the design discussion: a child prop borrowing the
// parent's body local across passes.
async fn lending_parent(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "parent");
    let title = String::from("borrowed title");
    let bump: Defer<()> = Defer::spawn(&cx, "bump", io);
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match bump.poll() {
            Deferred::Pending => out.push_str("[p1]"),
            Deferred::Ready(_) => out.push_str("[p2]"),
        }
        children.advance(&cx, &mut out, "child", |id| {
            Box::pin(borrowing_child(cx.clone(), id, &title))
        })?;
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn borrowing_child(cx: Cx, me: CompId, s: &str) -> Result {
    let mount = cx.register(me, "child");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        interp(&mut out, || format!("<b>{s}</b>"));
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn child_prop_borrows_parent_body_local_across_passes() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(lending_parent(cx.clone(), ROOT, io)));

    let p1 = driver.next_pass();
    assert_eq!(p1.html, "[p1]<b>borrowed title</b>");
    fire.fire();
    let p2 = driver.next_pass();
    // The child re-rendered reading the borrow, proving it stayed valid.
    assert_eq!(p2.html, "[p2]<b>borrowed title</b>");
    assert_eq!(cx.log_count("body:child"), 1);
    assert_eq!(cx.log_count("render:child"), 2);
}

// Two levels of frame borrows: the child derives its own local from a borrowed
// prop and lends that to a grandchild.
async fn deep_parent(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "parent");
    let title = String::from("root");
    let bump: Defer<()> = Defer::spawn(&cx, "bump", io);
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        let _ = bump.poll();
        children.advance(&cx, &mut out, "child", |id| {
            Box::pin(deep_child(cx.clone(), id, &title))
        })?;
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn deep_child(cx: Cx, me: CompId, s: &str) -> Result {
    let mount = cx.register(me, "child");
    let derived = format!("{s}-derived");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        children.advance(&cx, &mut out, "grand", |id| {
            Box::pin(deep_grandchild(cx.clone(), id, &derived))
        })?;
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn deep_grandchild(cx: Cx, me: CompId, s: &str) -> Result {
    let mount = cx.register(me, "grand");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        interp(&mut out, || format!("({s})"));
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn borrow_chain_holds_through_two_levels_of_nesting() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(deep_parent(cx.clone(), ROOT, io)));

    assert_eq!(driver.next_pass().html, "(root-derived)");
    fire.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.html, "(root-derived)");
    assert_eq!(p2.polls, 1);
    assert_eq!(cx.log_count("body:child"), 1);
    assert_eq!(cx.log_count("body:grand"), 1);
    assert_eq!(cx.log_count("render:grand"), 2);
}

// A layout: catches its slot's error, keeps its other children alive, and
// renders branded error UI in place of the slot.
async fn layout(cx: Cx, me: CompId, slot_io: Trigger) -> Result {
    let mount = cx.register(me, "layout");
    let mut slot_io = Some(slot_io);
    let mut failed: Option<Error> = None;
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        out.push_str("<nav>");
        children.advance(&cx, &mut out, "nav", |id| {
            Box::pin(nav_widget(cx.clone(), id))
        })?;
        out.push_str("</nav>");
        match &failed {
            Some(error) => interp(&mut out, || format!("<h1>error: {error}</h1>")),
            None => {
                let io = &mut slot_io;
                let slot = children.advance_catching(&cx, &mut out, "slot", |id| {
                    Box::pin(grid(cx.clone(), id, io.take().unwrap()))
                });
                if let Err(error) = slot {
                    interp(&mut out, || format!("<h1>error: {error}</h1>"));
                    failed = Some(error);
                }
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn nav_widget(cx: Cx, me: CompId) -> Result {
    let mount = cx.register(me, "nav");
    let mut children = Children::new();
    loop {
        cx.finish_render(&mount, String::from("menu"));
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

// Defers a Result and propagates the deferred error with `?` on a later pass.
async fn grid(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "grid");
    let data: Defer<Result<Vec<String>, String>> = Defer::spawn(&cx, "grid-data", async move {
        io.await;
        Err(String::from("db down"))
    });
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match data.poll() {
            Deferred::Pending => out.push_str("<div class=skeleton></div>"),
            Deferred::Ready(result) => {
                let list = result.as_ref().map_err(|e| Error::msg(e.clone()))?;
                for item in list {
                    interp(&mut out, || format!("<div>{item}</div>"));
                }
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn deferred_error_on_a_later_pass_is_caught_by_the_layout() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(layout(cx.clone(), ROOT, io)));

    let p1 = driver.next_pass();
    assert!(p1.html.contains("skeleton"));
    assert!(p1.html.contains("<nav>menu</nav>"));

    fire.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.pass, 2);
    assert!(p2.html.contains("<h1>error: db down</h1>"));
    assert!(
        p2.html.contains("<nav>menu</nav>"),
        "siblings outside the slot survive"
    );
    assert!(p2.page_error.is_none());
    assert_eq!(
        cx.log_count("drop:grid"),
        1,
        "the errored subtree was dropped"
    );
    assert_eq!(cx.log_count("body:grid"), 1);
}

// A slot whose body fails immediately: the error is caught on the first pass.
async fn doomed(cx: Cx, me: CompId, _io: Trigger) -> Result {
    let _mount = cx.register(me, "doomed");
    Err(Error::msg("boom at birth"))
}

#[test]
fn body_error_at_birth_is_caught_on_the_first_pass() {
    let cx = Cx::new();
    let (io, _fire) = trigger();
    async fn doomed_layout(cx: Cx, me: CompId, io: Trigger) -> Result {
        let mount = cx.register(me, "layout");
        let mut io = Some(io);
        let mut failed: Option<Error> = None;
        let mut children = Children::new();
        loop {
            let mut out = String::new();
            match &failed {
                Some(error) => interp(&mut out, || format!("<h1>error: {error}</h1>")),
                None => {
                    let slot_io = &mut io;
                    let slot = children.advance_catching(&cx, &mut out, "slot", |id| {
                        Box::pin(doomed(cx.clone(), id, slot_io.take().unwrap()))
                    });
                    if let Err(error) = slot {
                        interp(&mut out, || format!("<h1>error: {error}</h1>"));
                        failed = Some(error);
                    }
                }
            }
            children.sweep(&cx);
            cx.finish_render(&mount, out);
            pass_boundary(&cx, &mount, &mut children).await?;
        }
    }
    let mut driver = Driver::new(cx.clone(), Box::pin(doomed_layout(cx.clone(), ROOT, io)));
    let p1 = driver.next_pass();
    assert_eq!(p1.pass, 1);
    assert!(p1.html.contains("<h1>error: boom at birth</h1>"));
    assert_eq!(cx.log_count("drop:doomed"), 1);
}

// A component born on a later pass whose body parks on I/O and then fails
// while the tree is suspended: the error unwinds the intermediate parent, is
// stashed at the catching layout, and an extra pass renders the fallback.
async fn stash_page(cx: Cx, me: CompId, d1: Trigger, inner_io: Trigger) -> Result {
    let mount = cx.register(me, "page");
    let first: Defer<()> = Defer::spawn(&cx, "first", d1);
    let mut inner_io = Some(inner_io);
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match first.poll() {
            Deferred::Pending => out.push_str("[page skeleton]"),
            Deferred::Ready(_) => {
                children.advance(&cx, &mut out, "sib", |id| {
                    Box::pin(nav_widget(cx.clone(), id))
                })?;
                let io = &mut inner_io;
                children.advance(&cx, &mut out, "inner", |id| {
                    Box::pin(failing_inner(cx.clone(), id, io.take().unwrap()))
                })?;
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn failing_inner(cx: Cx, me: CompId, io: Trigger) -> Result {
    let _mount = cx.register(me, "inner");
    io.await;
    Err(Error::msg("late boom"))
}

async fn stash_layout(cx: Cx, me: CompId, d1: Trigger, inner_io: Trigger) -> Result {
    let mount = cx.register(me, "layout");
    let mut props = Some((d1, inner_io));
    let mut failed: Option<Error> = None;
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match &failed {
            Some(error) => interp(&mut out, || format!("<h1>error: {error}</h1>")),
            None => {
                let p = &mut props;
                let slot = children.advance_catching(&cx, &mut out, "slot", |id| {
                    let (d1, inner_io) = p.take().unwrap();
                    Box::pin(stash_page(cx.clone(), id, d1, inner_io))
                });
                if let Err(error) = slot {
                    interp(&mut out, || format!("<h1>error: {error}</h1>"));
                    failed = Some(error);
                }
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn mid_suspension_error_is_stashed_and_rendered_on_an_extra_pass() {
    let cx = Cx::new();
    let (d1, fire_d1) = trigger();
    let (inner_io, fire_inner) = trigger();
    let mut driver = Driver::new(
        cx.clone(),
        Box::pin(stash_layout(cx.clone(), ROOT, d1, inner_io)),
    );

    let p1 = driver.next_pass();
    assert!(p1.html.contains("[page skeleton]"));

    fire_d1.fire();
    driver.begin_pass();
    assert!(driver.pump().is_none());
    assert!(
        driver.try_seal().is_none(),
        "pass 2 waits on the parked birth"
    );

    fire_inner.fire();
    assert!(
        driver.pump().is_none(),
        "the error is stashed, not a page error"
    );
    assert!(
        driver.try_seal().is_none(),
        "a caught error blocks the seal"
    );

    driver.begin_pass();
    assert!(driver.pump().is_none());
    let p3 = driver.try_seal().expect("the error pass seals");
    assert_eq!(p3.pass, 3);
    assert!(p3.html.contains("<h1>error: late boom</h1>"));
    // The whole slot subtree died: the failing component, its healthy sibling
    // (the nav widget invoked at the "sib" key), and the intermediate page.
    assert_eq!(cx.log_count("drop:inner"), 1);
    assert_eq!(cx.log_count("drop:page"), 1);
    assert_eq!(cx.log_count("drop:nav"), 1);
    assert_eq!(cx.log_count("drop:layout"), 0);
}

// An error with no catcher above it becomes the whole page error.
async fn fatal_root(cx: Cx, me: CompId) -> Result {
    let _mount = cx.register(me, "root");
    Err(Error::msg("fatal"))
}

#[test]
fn uncaught_error_becomes_a_page_error() {
    let cx = Cx::new();
    let mut driver = Driver::new(cx.clone(), Box::pin(fatal_root(cx.clone(), ROOT)));
    let p1 = driver.next_pass();
    assert_eq!(p1.page_error, Some(Error::msg("fatal")));
}

// The error handled at the source: matching `Ready` on the deferred Result
// instead of propagating with `?`. Nothing unwinds, nothing is dropped.
async fn local_handler(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "local");
    let data: Defer<Result<String, String>> = Defer::spawn(&cx, "data", async move {
        io.await;
        Err(String::from("unavailable"))
    });
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match data.poll() {
            Deferred::Pending => out.push_str("[skeleton]"),
            Deferred::Ready(Ok(value)) => interp(&mut out, || value.clone()),
            Deferred::Ready(Err(error)) => {
                interp(&mut out, || format!("<p>could not load: {error}</p>"))
            }
        }
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn deferred_error_can_be_handled_at_the_source() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(local_handler(cx.clone(), ROOT, io)));

    driver.next_pass();
    fire.fire();
    let p2 = driver.next_pass();
    assert!(p2.html.contains("<p>could not load: unavailable</p>"));
    assert!(p2.page_error.is_none());
    assert_eq!(cx.log_count("drop:"), 0);
}

// A `Drop` impl that reads a borrowed prop, both at eviction and at the end of
// the request. This is the teardown soundness case an arena cannot offer
// safely; the frame's compiler managed drop order makes it just work.
struct Audit<'a> {
    label: &'static str,
    s: &'a str,
    cx: Cx,
}

impl Drop for Audit<'_> {
    fn drop(&mut self) {
        self.cx.log(format!("audit:{}:{}", self.label, self.s));
    }
}

async fn audited(cx: Cx, me: CompId, label: &'static str, s: &str) -> Result {
    let mount = cx.register(me, "audited");
    let audit = Audit {
        label,
        s,
        cx: cx.clone(),
    };
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        interp(&mut out, || format!("[{}:{}]", audit.label, audit.s));
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

async fn audit_parent(cx: Cx, me: CompId, io: Trigger) -> Result {
    let mount = cx.register(me, "parent");
    let switch: Defer<()> = Defer::spawn(&cx, "switch", io);
    let title = String::from("The menu");
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        match switch.poll() {
            Deferred::Pending => {
                children.advance(&cx, &mut out, "evictee", |id| {
                    Box::pin(audited(cx.clone(), id, "evictee", &title))
                })?;
            }
            Deferred::Ready(_) => {
                children.advance(&cx, &mut out, "keeper", |id| {
                    Box::pin(audited(cx.clone(), id, "keeper", &title))
                })?;
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn arm_switch_evicts_the_orphan_and_drop_reads_the_borrowed_prop() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(audit_parent(cx.clone(), ROOT, io)));

    let p1 = driver.next_pass();
    assert_eq!(p1.html, "[evictee:The menu]");

    fire.fire();
    let p2 = driver.next_pass();
    assert_eq!(p2.html, "[keeper:The menu]");
    // Eviction dropped the orphan, and its Drop read the borrowed prop while
    // the parent's local was still alive.
    assert!(
        cx.log_snapshot()
            .contains(&String::from("audit:evictee:The menu"))
    );
    assert_eq!(
        cx.log_count("body:audited"),
        2,
        "the two arms are two instances"
    );

    // End of request: dropping the driver drops the whole tree; the surviving
    // child's Drop also reads the borrow, in compiler managed order.
    drop(driver);
    assert!(
        cx.log_snapshot()
            .contains(&String::from("audit:keeper:The menu"))
    );
    assert_eq!(cx.log_count("drop:parent"), 1);
}

// Retention across extra passes: keyed children are advanced, not reborn.
async fn retained_menu(cx: Cx, me: CompId, io1: Trigger, io2: Trigger) -> Result {
    let mount = cx.register(me, "menu");
    let drinks: Defer<Vec<String>> = Defer::spawn(&cx, "drinks", async move {
        io1.await;
        vec![String::from("mojito"), String::from("mai-tai")]
    });
    let bump: Defer<()> = Defer::spawn(&cx, "bump", io2);
    let mut children = Children::new();
    loop {
        let mut out = String::new();
        let _ = bump.poll();
        match drinks.poll() {
            Deferred::Pending => out.push_str("[skeleton]"),
            Deferred::Ready(list) => {
                for name in list {
                    children.advance(&cx, &mut out, name, |id| {
                        Box::pin(card(cx.clone(), id, name))
                    })?;
                }
            }
        }
        children.sweep(&cx);
        cx.finish_render(&mount, out);
        pass_boundary(&cx, &mount, &mut children).await?;
    }
}

#[test]
fn keyed_children_are_retained_across_passes_and_settled_passes_are_one_poll() {
    let cx = Cx::new();
    let (io1, fire1) = trigger();
    let (io2, fire2) = trigger();
    let mut driver = Driver::new(
        cx.clone(),
        Box::pin(retained_menu(cx.clone(), ROOT, io1, io2)),
    );

    driver.next_pass();
    fire1.fire();
    let p2 = driver.next_pass();
    assert!(p2.html.contains("MOJITO"));

    fire2.fire();
    let p3 = driver.next_pass();
    assert_eq!(p3.pass, 3);
    // The cards were advanced, not reborn: bodies once each across 3 passes.
    assert_eq!(cx.log_count("body:card"), 2);
    assert_eq!(
        cx.log_count("render:card"),
        4,
        "each card rendered on passes 2 and 3"
    );
    // A settled tree is a single poll per pass.
    assert_eq!(p3.polls, 1);
    // Deterministic renders: nothing changed on pass 3, so nothing would swap.
    assert_eq!(p3.changed, Vec::<&str>::new());
}

// A deferred load that never resolves: skeletons persist, passes stay bounded.
#[test]
fn unresolved_defer_keeps_the_skeleton_under_a_pass_cap() {
    let cx = Cx::new();
    let (io, _fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(menu(cx.clone(), ROOT, io)));

    let reports = driver.run_to_completion(3);
    assert_eq!(reports.len(), 3, "the cap bounds the passes");
    assert!(reports.iter().all(|r| r.html.contains("skeleton")));
    assert_eq!(
        cx.outstanding_deferred(),
        1,
        "the load is still outstanding at the cap"
    );
    assert_eq!(
        reports[1].changed,
        Vec::<&str>::new(),
        "identical renders change nothing"
    );
}

// The changed set: a pass ships only components whose output moved.
#[test]
fn only_components_whose_output_changed_are_reported() {
    let cx = Cx::new();
    let (io, fire) = trigger();
    let mut driver = Driver::new(cx.clone(), Box::pin(lending_parent(cx.clone(), ROOT, io)));

    let p1 = driver.next_pass();
    assert_eq!(p1.changed, vec!["child", "parent"]);
    fire.fire();
    let p2 = driver.next_pass();
    // Only the parent's own slot changed on pass 2; the child's output is
    // byte identical and would never be re-sent.
    assert_eq!(p2.changed, vec!["parent"]);
}
