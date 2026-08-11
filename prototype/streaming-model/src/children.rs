use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use crate::cx::{CompId, CompState, Cx, Mount};
use crate::error::{Error, Result};

/// The type of a stored component future.
///
/// `'f` is the enclosing frame: child futures may borrow the parent's body
/// locals, which is the load bearing pattern this prototype validates.
pub type CompFut<'f> = Pin<Box<dyn Future<Output = Result> + 'f>>;

struct ChildEntry<'f> {
    id: CompId,
    fut: Option<CompFut<'f>>,
    catching: bool,
    stashed: Option<Error>,
    advanced_pass: u64,
}

/// A component's children, keyed the way the identity system keys invocations.
///
/// This is a frame local of the generated render loop. Entries hold the child
/// futures across passes; dropping an entry is eviction.
pub struct Children<'f> {
    map: HashMap<String, ChildEntry<'f>>,
}

impl<'f> Children<'f> {
    pub fn new() -> Self {
        Children {
            map: HashMap::new(),
        }
    }

    /// Advances the child at `key` for the current pass, creating it on first
    /// reach. Emits the child's output marker into `out`. A child error is
    /// returned so the caller can propagate it with `?`.
    pub fn advance(
        &mut self,
        cx: &Cx,
        out: &mut String,
        key: &str,
        mk: impl FnOnce(CompId) -> CompFut<'f>,
    ) -> Result {
        self.advance_inner(cx, out, key, false, mk)
    }

    /// Like [`Children::advance`], but marks the child as caught: an error that
    /// completes the child while the parent is suspended is stashed here and
    /// returned from the next advance, instead of unwinding the parent. This is
    /// the layout `slot` behavior.
    pub fn advance_catching(
        &mut self,
        cx: &Cx,
        out: &mut String,
        key: &str,
        mk: impl FnOnce(CompId) -> CompFut<'f>,
    ) -> Result {
        self.advance_inner(cx, out, key, true, mk)
    }

    fn advance_inner(
        &mut self,
        cx: &Cx,
        out: &mut String,
        key: &str,
        catching: bool,
        mk: impl FnOnce(CompId) -> CompFut<'f>,
    ) -> Result {
        let pass = cx.pass();
        let entry = self.map.entry(key.to_string()).or_insert_with(|| {
            let id = cx.fresh_id();
            ChildEntry {
                id,
                fut: Some(mk(id)),
                catching,
                stashed: None,
                advanced_pass: 0,
            }
        });
        assert!(
            entry.advanced_pass < pass,
            "child {key:?} advanced twice in one pass"
        );
        entry.advanced_pass = pass;
        let id = entry.id;

        if let Some(error) = entry.stashed.take() {
            cx.consume_stashed();
            self.map.remove(key);
            return Err(error);
        }

        let outcome = poll_child(cx, entry);
        match outcome {
            Poll::Ready(Err(error)) => {
                self.map.remove(key);
                return Err(error);
            }
            Poll::Ready(Ok(())) => unreachable!("component future completed without an error"),
            Poll::Pending => {}
        }

        out.push_str(&marker(id));
        Ok(())
    }

    /// Evicts every child not advanced during the current pass. The generated
    /// render calls this after the view code ran, so a child orphaned by a
    /// branch switch is dropped.
    pub fn sweep(&mut self, cx: &Cx) {
        let pass = cx.pass();
        self.map.retain(|_, entry| {
            if entry.advanced_pass == pass {
                true
            } else {
                cx.birth_done(entry.id);
                false
            }
        });
    }
}

impl Default for Children<'_> {
    fn default() -> Self {
        Children::new()
    }
}

/// Polls one child once and updates the birth bookkeeping.
fn poll_child(cx: &Cx, entry: &mut ChildEntry<'_>) -> Poll<Result> {
    let Some(fut) = entry.fut.as_mut() else {
        return Poll::Pending;
    };
    let waker = Waker::noop();
    let mut ctx = Context::from_waker(waker);
    match fut.as_mut().poll(&mut ctx) {
        Poll::Ready(result) => {
            cx.birth_done(entry.id);
            entry.fut = None;
            Poll::Ready(result)
        }
        Poll::Pending => {
            if cx.rendered_this_pass(entry.id) {
                cx.birth_done(entry.id);
            } else {
                cx.birth_started(entry.id);
            }
            Poll::Pending
        }
    }
}

fn marker(id: CompId) -> String {
    format!("<!--c:{id}-->")
}

/// Suspends a component until the next pass, driving its children while it
/// waits.
///
/// This is the boundary the generated render loop awaits. While suspended it
/// keeps polling the children so a birth parked on I/O, at any depth, still
/// makes progress; a child that completes with an error either unwinds through
/// this future's `Result` or is stashed if the child was advanced catching.
pub fn pass_boundary<'a, 'f>(
    cx: &Cx,
    mount: &Mount,
    children: &'a mut Children<'f>,
) -> PassBoundary<'a, 'f> {
    PassBoundary {
        cx: cx.clone(),
        state: mount.state(),
        children,
    }
}

pub struct PassBoundary<'a, 'f> {
    cx: Cx,
    state: Rc<CompState>,
    children: &'a mut Children<'f>,
}

impl Future for PassBoundary<'_, '_> {
    type Output = Result;

    fn poll(self: Pin<&mut Self>, _ctx: &mut Context<'_>) -> Poll<Result> {
        let this = self.get_mut();
        if this.cx.pass() > this.state.rendered_pass() {
            return Poll::Ready(Ok(()));
        }
        for entry in this.children.map.values_mut() {
            if entry.stashed.is_some() {
                continue;
            }
            if let Poll::Ready(Err(error)) = poll_child(&this.cx, entry) {
                if entry.catching {
                    entry.stashed = Some(error);
                    this.cx.note_stashed();
                } else {
                    return Poll::Ready(Err(error));
                }
            }
        }
        Poll::Pending
    }
}
