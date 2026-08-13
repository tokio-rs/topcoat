//! The driver: composes the page into one live render, polls it as a
//! single task, sends the first paint when the root cell delivers, and a
//! chunk whenever a swap marks the buffer dirty.
//!
//! Topcoat-level API under check (DESIGN.md "The Driver"):
//!
//! - One task, no spawning: the render is one future, polled to the
//!   first paint and then until nothing can change anymore.
//! - The scope installed around every poll; the whole root future is
//!   asserted `Send` even though it reaches shared state with no locks.
//! - Sweep-until-quiescent: re-poll while a pass made progress; a real
//!   driver parks on the task waker otherwise (the spike's stand-in
//!   `Slow` futures complete by being re-polled, so the loop spins).
//! - Chunks: first paint at root delivery, then one chunk per dirty
//!   sweep, coalescing same-pass swaps for free.
//! - An uncaught error transition surfaces as the root future's `Err`,
//!   where the framework error view would swap in.

#![forbid(unsafe_code)]

mod page;
mod scope;
mod set;

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use page::page;
use scope::RenderScope;
use set::{Cx, Fill};

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn assert_send<T: Send>(_: &T) {}

fn run_scenario(name: &str, fail_orders: bool) {
    println!("==== scenario: {name} ====");
    let cx = Cx { fail_orders };
    let mut scope = RenderScope::new();

    // The router's side of the fill: the root cell the page delivers to.
    let root_cell = scope::install(&mut scope, || scope::with(|s| s.new_cell(true)));
    let root = page(&cx, Fill::for_cell(root_cell));

    // The render is Send although it reaches the scope with no locks:
    // the scope travels with this driver, not inside the future.
    assert_send(&root);

    let mut root = Box::pin(root);
    let waker = Waker::from(Arc::new(NoopWake));
    let mut task = Context::from_waker(&waker);

    let mut sent_first = false;
    let mut chunk = 0;
    let mut polls = 0u32;

    loop {
        let poll = scope::install(&mut scope, || {
            scope::with(|s| s.progress = false);
            root.as_mut().poll(&mut task)
        });
        polls += 1;
        assert!(polls < 10_000, "livelock: the render never settled");

        if !sent_first {
            if let Some(state) = &scope.cells[root_cell].delivered {
                sent_first = true;
                scope.dirty = false;
                chunk += 1;
                match state {
                    Ok(view) => {
                        let view = *view;
                        println!("-- chunk {chunk} (first paint) --\n{}", scope.render(view));
                    }
                    Err(e) => println!("-- failed before first paint: {e} --"),
                }
            }
        } else if scope.dirty {
            scope.dirty = false;
            if let Some(Ok(view)) = &scope.cells[root_cell].delivered {
                let view = *view;
                chunk += 1;
                println!("-- chunk {chunk} (swap) --\n{}", scope.render(view));
            }
        }

        match poll {
            Poll::Ready(Ok(())) => {
                println!("-- stream closed after {polls} polls --\n");
                break;
            }
            Poll::Ready(Err(e)) => {
                println!("-- root error transition: {e} (framework error view swaps in) --\n");
                break;
            }
            Poll::Pending => {}
        }
    }
}

fn main() {
    run_scenario("happy path", false);
    run_scenario("orders fail mid-stream", true);
}
