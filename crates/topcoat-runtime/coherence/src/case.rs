use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use topcoat::runtime::{Expr, Surrogate};

use crate::{Engine, Observe, Outcome};

/// An expression and its Rust outcome, ready for comparison in JavaScript.
pub struct Case {
    expression: &'static str,
    source: String,
    rust: Outcome,
    invoke: bool,
}

impl Case {
    /// Creates a case from a directly evaluated expression.
    pub fn evaluated<T: Observe>(expression: &'static str, compiled: Expr<T>) -> Self {
        let (value, js) = compiled.into_evaluated_and_js();
        Self {
            expression,
            source: js.to_source(),
            rust: Outcome::Return(value.observe()),
            invoke: false,
        }
    }

    /// Invokes a compiled closure, capturing a Rust panic as an outcome.
    pub fn deferred<F, S>(expression: &'static str, compiled: Expr<F>) -> Self
    where
        F: FnOnce() -> S,
        S: Surrogate,
        S::Real: Observe,
    {
        let (run, js) = compiled.into_evaluated_and_js();
        let source = js.to_source();
        let rust = match catch_unwind(AssertUnwindSafe(run)) {
            Ok(value) => Outcome::Return(value.into_real().observe()),
            Err(payload) => {
                let message = payload.downcast_ref::<String>().cloned().or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_owned())
                });
                Outcome::Panic(message.unwrap_or_else(|| "non-string panic payload".to_owned()))
            }
        };
        Self {
            expression,
            source,
            rust,
            invoke: true,
        }
    }

    /// Executes JavaScript and compares its outcome with Rust's.
    ///
    /// # Errors
    ///
    /// Returns an error for unequal outcomes or a JavaScript execution failure.
    pub fn check(&self) -> Result<()> {
        match self.javascript() {
            Ok(javascript) if self.rust.agrees_with(&javascript) => Ok(()),
            Ok(javascript) => bail!(
                "expression: {}\nRust: {:#?}\nJavaScript: {javascript:#?}\ngenerated JavaScript:\n{}",
                self.expression,
                self.rust,
                self.source,
            ),
            Err(error) => bail!(
                "expression: {}\nRust: {:#?}\nJavaScript execution failed: {error:#}\ngenerated JavaScript:\n{}",
                self.expression,
                self.rust,
                self.source,
            ),
        }
    }

    fn javascript(&self) -> Result<Outcome> {
        thread_local! {
            static ENGINE: RefCell<Engine> = RefCell::new(Engine::new(Duration::from_secs(5)));
        }

        let source = serde_json::to_string(&self.source)?;
        let script = format!(
            "{}\nTopcoatCoherence.execute({source}, {});",
            include_str!("../../browser/dist/coherence.js"),
            self.invoke,
        );
        let json = ENGINE.with(|engine| engine.borrow_mut().evaluate(&script))?;
        serde_json::from_str(&json).context("invalid JavaScript observation")
    }

    /// Checks that a named discrepancy still has its recorded outcomes.
    ///
    /// # Errors
    ///
    /// Returns an error for missing baselines, changed outcomes, execution
    /// failures, or an unexpected agreement between Rust and JavaScript.
    pub fn check_known(&self, id: &str) -> Result<()> {
        let baseline: Baseline = toml::from_str(include_str!("../known-mismatches.toml"))?;
        let matches: Vec<_> = baseline
            .mismatch
            .iter()
            .filter(|item| item.id == id)
            .collect();
        anyhow::ensure!(matches.len() == 1, "expected exactly one baseline for {id}");
        let expected = matches[0];
        let javascript = self.javascript().with_context(|| {
            format!(
                "known mismatch {id}: {}\ngenerated JavaScript:\n{}",
                self.expression, self.source
            )
        })?;
        anyhow::ensure!(
            !self.rust.agrees_with(&javascript),
            "known mismatch {id} now passes; remove its baseline: {}",
            expected.reason,
        );
        anyhow::ensure!(
            self.rust == expected.rust && javascript == expected.javascript,
            "known mismatch {id} changed: {}\nexpected Rust: {:#?}\nexpected JavaScript: {:#?}\nactual Rust: {:#?}\nactual JavaScript: {javascript:#?}\ngenerated JavaScript:\n{}",
            expected.reason,
            expected.rust,
            expected.javascript,
            self.rust,
            self.source,
        );
        Ok(())
    }

    /// Asserts that Rust and JavaScript agree.
    ///
    /// # Panics
    ///
    /// Panics with both outcomes and generated source if the case fails.
    #[track_caller]
    pub fn assert(&self) {
        if let Err(error) = self.check() {
            panic!("{error:#}");
        }
    }

    /// Asserts that a known mismatch has exactly its recorded outcomes.
    ///
    /// # Panics
    ///
    /// Panics if the baseline is absent, the outcome changes, or the case passes.
    #[track_caller]
    pub fn assert_known(&self, id: &str) {
        if let Err(error) = self.check_known(id) {
            panic!("{error:#}");
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    mismatch: Vec<KnownMismatch>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownMismatch {
    id: String,
    reason: String,
    rust: Outcome,
    javascript: Outcome,
}

/// Runs an expression on both sides and asserts equal outcomes.
///
/// The default form wraps the expression in a closure to capture panics. Use
/// `coherent!(direct => expression)` to exercise direct expression lowering.
#[macro_export]
macro_rules! coherent {
    (known $id:literal => $($expression:tt)+) => {
        $crate::Case::deferred(stringify!($($expression)+), $crate::compile!(|| $($expression)+)).assert_known($id)
    };
    (direct => $($expression:tt)+) => {
        $crate::Case::evaluated(stringify!($($expression)+), $crate::compile!($($expression)+)).assert()
    };
    ($($expression:tt)+) => {
        $crate::Case::deferred(stringify!($($expression)+), $crate::compile!(|| $($expression)+)).assert()
    };
}
