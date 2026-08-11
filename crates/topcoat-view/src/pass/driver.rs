use std::{
    collections::HashMap,
    future::{Future, poll_fn},
    pin::Pin,
    task::{Context, Poll},
};

use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{
    identity::Identity,
    pass::{
        children::InstallFuture,
        scope::{PassScope, PassState, Slot},
    },
};

/// Drives one request's render passes over the root component.
///
/// A pass snapshots the deferred completions, advances the pass counter, and
/// polls the root until the pass seals: every live component rendered, no
/// birth in flight, no caught error waiting. Everything runs on the task
/// polling the driver; nothing is spawned.
pub struct Driver<'f> {
    root: Option<Pin<Box<dyn Future<Output = Result<()>> + Send + 'f>>>,
    state: Option<Box<PassState>>,
    deferred: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>,
    prev: HashMap<u128, String>,
    rendering: bool,
    pumps: u32,
    done: bool,
}

/// What one sealed pass produced.
#[derive(Debug)]
pub struct PassReport {
    /// The pass number, starting at one.
    pub pass: u64,
    /// The assembled document after this pass.
    pub html: String,
    /// Identity hashes of components whose output changed this pass, sorted.
    pub changed: Vec<u128>,
    /// How many root polls the pass needed.
    pub polls: u32,
    /// Set when the root itself completed with an error: the whole page
    /// error case, with no layout left to catch it.
    pub page_error: Option<Error>,
    /// The response status renders have declared so far.
    #[cfg(feature = "http")]
    pub status_code: Option<http::StatusCode>,
    /// The response headers renders have declared so far.
    #[cfg(feature = "http")]
    pub headers: http::HeaderMap,
}

impl<'f> Driver<'f> {
    /// Creates the driver for one request over the root component's future.
    pub fn new<F>(cx: Cx, root: F) -> Self
    where
        F: Future<Output = Result<()>> + Send + 'f,
    {
        Driver {
            root: Some(Box::pin(InstallFuture::new(Identity::ROOT, root))),
            state: Some(Box::new(PassState::new(cx))),
            deferred: Vec::new(),
            prev: HashMap::new(),
            rendering: false,
            pumps: 0,
            done: false,
        }
    }

    /// Advances the request toward its next sealed pass.
    ///
    /// `Ready(Some(report))` is a sealed pass. `Ready(None)` ends the
    /// stream: everything rendered and no deferred load is outstanding.
    /// `Pending` means the pass is waiting on I/O; the task is woken when it
    /// can progress.
    pub fn poll_next_pass(&mut self, ctx: &mut Context<'_>) -> Poll<Option<PassReport>> {
        loop {
            if self.done {
                return Poll::Ready(None);
            }
            if !self.rendering {
                let first = self.pass() == 0;
                let completed = self.poll_deferred(ctx);
                if !first && completed == 0 {
                    if self.deferred.is_empty() {
                        self.done = true;
                        return Poll::Ready(None);
                    }
                    return Poll::Pending;
                }
                self.begin_pass();
            }
            if let Some(error) = self.pump(ctx) {
                self.done = true;
                return Poll::Ready(Some(PassReport {
                    pass: self.pass(),
                    html: String::new(),
                    changed: Vec::new(),
                    polls: self.pumps,
                    page_error: Some(error),
                    #[cfg(feature = "http")]
                    status_code: None,
                    #[cfg(feature = "http")]
                    headers: http::HeaderMap::new(),
                }));
            }
            self.drain_handoff();
            if let Some(report) = self.try_seal() {
                self.rendering = false;
                return Poll::Ready(Some(report));
            }
            if self.quiescent() && self.stashed() > 0 {
                self.begin_pass();
                continue;
            }
            return Poll::Pending;
        }
    }

    /// The next sealed pass, or `None` once the stream is over.
    pub async fn next_pass(&mut self) -> Option<PassReport> {
        poll_fn(|ctx| self.poll_next_pass(ctx)).await
    }

    /// Renders to completion without streaming: runs every pass and returns
    /// the final document. The blocking mode for clients that cannot stream.
    ///
    /// # Errors
    ///
    /// An error nothing caught: the root completed with it, so there is no
    /// document to return.
    pub async fn render_blocking(&mut self) -> Result<String> {
        let mut html = String::new();
        while let Some(report) = self.next_pass().await {
            if let Some(error) = report.page_error {
                return Err(error);
            }
            html = report.html;
        }
        Ok(html)
    }

    /// How many deferred loads are still in flight.
    #[must_use]
    pub fn outstanding_deferred(&self) -> usize {
        self.deferred.len()
    }

    fn pass(&self) -> u64 {
        self.state
            .as_ref()
            .expect("the pass state is held between polls")
            .pass
    }

    fn stashed(&self) -> usize {
        self.state
            .as_ref()
            .expect("the pass state is held between polls")
            .stashed
    }

    fn begin_pass(&mut self) {
        let state = self
            .state
            .as_mut()
            .expect("the pass state is held between polls");
        state.pass += 1;
        self.pumps = 0;
        self.rendering = true;
    }

    /// Polls every outstanding deferred future once and returns how many
    /// completed. Runs only between passes, so a pass observes a consistent
    /// snapshot of completions.
    fn poll_deferred(&mut self, ctx: &mut Context<'_>) -> usize {
        let mut completed = 0;
        self.deferred
            .retain_mut(|fut| match fut.as_mut().poll(ctx) {
                Poll::Ready(()) => {
                    completed += 1;
                    false
                }
                Poll::Pending => true,
            });
        completed
    }

    /// Polls the root once under the installed pass state. Returns the page
    /// error if the root completed.
    fn pump(&mut self, ctx: &mut Context<'_>) -> Option<Error> {
        self.pumps += 1;
        if let Some(state) = self.state.as_mut() {
            state.waker = Some(ctx.waker().clone());
        }
        let root = self
            .root
            .as_mut()
            .expect("the request already ended with a page error");
        let result = {
            let _scope = PassScope::install(&mut self.state);
            root.as_mut().poll(ctx)
        };
        match result {
            Poll::Pending => None,
            Poll::Ready(Err(error)) => {
                self.root = None;
                Some(error)
            }
            Poll::Ready(Ok(())) => {
                panic!("the root component completed without suspending or failing")
            }
        }
    }

    fn drain_handoff(&mut self) {
        let state = self
            .state
            .as_mut()
            .expect("the pass state is held between polls");
        self.deferred.append(&mut state.deferred);
    }

    fn quiescent(&self) -> bool {
        let state = self
            .state
            .as_ref()
            .expect("the pass state is held between polls");
        state.births.is_empty()
            && state
                .slots
                .values()
                .all(|slot| slot.rendered_pass == state.pass)
    }

    fn try_seal(&mut self) -> Option<PassReport> {
        if !self.quiescent() || self.stashed() > 0 {
            return None;
        }
        let state = self
            .state
            .as_ref()
            .expect("the pass state is held between polls");
        let mut changed: Vec<u128> = state
            .slots
            .iter()
            .filter(|(hash, slot)| self.prev.get(*hash) != Some(&slot.html))
            .map(|(hash, _)| *hash)
            .collect();
        changed.sort_unstable();
        let html = assemble(Identity::ROOT.hash(), &state.slots);
        let report = PassReport {
            pass: state.pass,
            html,
            changed,
            polls: self.pumps,
            page_error: None,
            #[cfg(feature = "http")]
            status_code: state.status_code,
            #[cfg(feature = "http")]
            headers: state.headers.clone(),
        };
        self.prev = state
            .slots
            .iter()
            .map(|(hash, slot)| (*hash, slot.html.clone()))
            .collect();
        Some(report)
    }
}

/// Expands a component's output slot, recursively substituting child markers.
fn assemble(hash: u128, slots: &HashMap<u128, Slot>) -> String {
    let Some(slot) = slots.get(&hash) else {
        return String::new();
    };
    let mut html = String::with_capacity(slot.html.len());
    let mut rest = slot.html.as_str();
    while let Some(start) = rest.find("<!--tc:") {
        html.push_str(&rest[..start]);
        let after = &rest[start + "<!--tc:".len()..];
        let end = after.find("-->").expect("unterminated child marker");
        let child = u128::from_str_radix(&after[..end], 16).expect("malformed child marker");
        html.push_str(&assemble(child, slots));
        rest = &after[end + "-->".len()..];
    }
    html.push_str(rest);
    html
}
