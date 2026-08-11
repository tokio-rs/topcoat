use std::collections::HashMap;
use std::task::{Context, Poll, Waker};

use crate::children::CompFut;
use crate::cx::{CompId, Cx};
use crate::error::Error;

/// The root component's id, assigned by convention.
pub const ROOT: CompId = 0;

/// Drives one request: owns the root component future and runs passes over it.
///
/// The driver is deliberately manual so tests control time. A pass is: poll the
/// deferred queue once (the snapshot), advance the pass counter, then poll the
/// root until every live component has rendered and no birth is in flight.
pub struct Driver<'f> {
    cx: Cx,
    root: Option<CompFut<'f>>,
    prev_outputs: HashMap<CompId, String>,
    pumps: u32,
}

/// What one sealed pass produced.
#[derive(Debug)]
pub struct PassReport {
    pub pass: u64,
    pub html: String,
    /// Names of live components whose output slot changed this pass, sorted.
    pub changed: Vec<&'static str>,
    /// How many root polls the pass needed.
    pub polls: u32,
    /// Set when the root itself completed with an error: the whole page error
    /// case, with no layout left to catch it.
    pub page_error: Option<Error>,
}

impl<'f> Driver<'f> {
    pub fn new(cx: Cx, root: CompFut<'f>) -> Self {
        Driver {
            cx,
            root: Some(root),
            prev_outputs: HashMap::new(),
            pumps: 0,
        }
    }

    /// Starts the next pass: snapshots deferred completions, then advances the
    /// pass counter.
    pub fn begin_pass(&mut self) {
        self.cx.poll_deferred();
        self.cx.advance_pass();
        self.pumps = 0;
    }

    /// Polls the root once. Returns the page error if the root completed.
    pub fn pump(&mut self) -> Option<Error> {
        let root = self
            .root
            .as_mut()
            .expect("request already ended with a page error");
        self.pumps += 1;
        let waker = Waker::noop();
        let mut ctx = Context::from_waker(waker);
        match root.as_mut().poll(&mut ctx) {
            Poll::Pending => None,
            Poll::Ready(Err(error)) => {
                self.root = None;
                Some(error)
            }
            Poll::Ready(Ok(())) => unreachable!("root component completed without an error"),
        }
    }

    /// Seals the pass if it is complete: every live component rendered, no
    /// birth in flight, no caught error waiting to be consumed.
    pub fn try_seal(&mut self) -> Option<PassReport> {
        if !self.quiescent() || self.cx.stashed() > 0 {
            return None;
        }
        let outputs = self.cx.outputs_snapshot();
        let names = self.cx.live_names();
        let mut changed: Vec<&'static str> = outputs
            .iter()
            .filter(|(id, html)| self.prev_outputs.get(id) != Some(html))
            .filter_map(|(id, _)| names.get(id).copied())
            .collect();
        changed.sort_unstable();
        let html = assemble(ROOT, &outputs);
        self.prev_outputs = outputs;
        Some(PassReport {
            pass: self.cx.pass(),
            html,
            changed,
            polls: self.pumps,
            page_error: None,
        })
    }

    /// Runs one full pass: begin, pump until sealed. A pass that is quiescent
    /// but holds a caught error automatically rolls into the next pass so the
    /// catcher can render, matching the design's mid-stream error flow.
    ///
    /// Panics if the pass stalls, which means a body is parked on a trigger the
    /// test has not fired; drive those scenarios with `begin_pass` and `pump`.
    pub fn next_pass(&mut self) -> PassReport {
        self.begin_pass();
        for _ in 0..64 {
            if let Some(error) = self.pump() {
                return PassReport {
                    pass: self.cx.pass(),
                    html: String::new(),
                    changed: Vec::new(),
                    polls: self.pumps,
                    page_error: Some(error),
                };
            }
            if let Some(report) = self.try_seal() {
                return report;
            }
            if self.quiescent() && self.cx.stashed() > 0 {
                self.begin_pass();
            }
        }
        panic!(
            "pass {} stalled: a body is parked on an unfired trigger",
            self.cx.pass()
        );
    }

    /// Runs passes until no deferred future is outstanding or the cap is hit.
    /// Returns the sealed reports, one per pass.
    pub fn run_to_completion(&mut self, max_passes: usize) -> Vec<PassReport> {
        let mut reports = Vec::new();
        for _ in 0..max_passes {
            let report = self.next_pass();
            let done = report.page_error.is_some() || self.cx.outstanding_deferred() == 0;
            reports.push(report);
            if done {
                break;
            }
        }
        reports
    }

    fn quiescent(&self) -> bool {
        self.cx.all_rendered() && self.cx.births_in_flight() == 0
    }
}

/// Expands a component's output slot, recursively substituting child markers.
fn assemble(id: CompId, outputs: &HashMap<CompId, String>) -> String {
    let Some(slot) = outputs.get(&id) else {
        return String::new();
    };
    let mut html = String::with_capacity(slot.len());
    let mut rest = slot.as_str();
    while let Some(start) = rest.find("<!--c:") {
        html.push_str(&rest[..start]);
        let after = &rest[start + "<!--c:".len()..];
        let end = after.find("-->").expect("unterminated child marker");
        let child: CompId = after[..end].parse().expect("malformed child marker");
        html.push_str(&assemble(child, outputs));
        rest = &after[end + "-->".len()..];
    }
    html.push_str(rest);
    html
}
