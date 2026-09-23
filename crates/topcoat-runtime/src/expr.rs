use std::cell::Cell;

use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter};

use crate::{Js, Surrogate, Surrogated};

/// A runtime expression: a value computed on the server, together with the
/// JavaScript that computes the same value in the browser.
///
/// The `expr!` macro and `$(...)`
/// in a `view!` body create these. An expression is static if its evaluation
/// read no signals. A static expression renders as its plain value, without
/// browser bindings or marker comments. Its JavaScript is still available,
/// for example for event handlers and shard arguments.
#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: Js,
    pub(crate) is_static: bool,
}

impl<T> Expr<T> {
    /// Runs `evaluate` and pairs its result with `js`, recording whether the
    /// evaluation read any signals.
    ///
    /// The evaluation is synchronous. The signals it reads must be the same
    /// signals the JavaScript depends on, since they decide whether the
    /// expression is static.
    #[inline]
    pub fn evaluate(evaluate: impl FnOnce() -> T, js: Js) -> Self {
        let _scope = ReadScope::enter();
        let evaluated = evaluate();
        Self {
            evaluated,
            js,
            is_static: SIGNAL_READ.get() == Some(false),
        }
    }

    /// Whether the expression read no signals during
    /// [`evaluate`](Self::evaluate), so it never needs to run again in the
    /// browser.
    #[inline]
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Splits the expression into its value and its JavaScript.
    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, Js) {
        (self.evaluated, self.js)
    }

    /// Returns the value for use inside another expression's evaluation,
    /// marking that evaluation as reading signals if this expression did.
    #[doc(hidden)]
    pub fn into_captured_value(self) -> T::Surrogate
    where
        T: Surrogated,
    {
        if !self.is_static {
            mark_signal_read();
        }
        self.evaluated.into_surrogate()
    }
}

impl<T> From<T> for Expr<T>
where
    T: Surrogated,
    T::Surrogate: Serialize,
{
    /// Creates a static expression from a value in the runtime vocabulary.
    ///
    /// # Panics
    ///
    /// Panics if the value's surrogate cannot be serialized.
    fn from(value: T) -> Self {
        let surrogate = value.into_surrogate();
        let js = Js::builder().surrogate(&surrogate).build();
        Self::evaluate(|| surrogate.into_real(), js)
    }
}

impl<T> NodeViewParts for Expr<T>
where
    T: NodeViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if self.is_static {
            self.evaluated.into_view_parts(cx, parts);
            return;
        }

        // <!-- ::topcoat::expr::start("<js>") -->
        //
        // The comment context seals the source, so a `"` inside it renders
        // as `&quot;` and the quotes stay unambiguous delimiters on the
        // client.
        parts.push_comment(|comment| {
            comment.push_promoted_str_unescaped(&"::topcoat::expr::start(\"");
            self.js.write(comment);
            comment.push_promoted_str_unescaped(&"\")");
        });
        self.evaluated.into_view_parts(cx, parts);
        parts.push_comment(|comment| {
            comment.push_promoted_str_unescaped(&"::topcoat::expr::end");
        });
    }
}

thread_local! {
    static SIGNAL_READ: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Records a read and returns whether an expression is being evaluated.
pub(crate) fn mark_signal_read() -> bool {
    if SIGNAL_READ.get().is_some() {
        SIGNAL_READ.set(Some(true));
        true
    } else {
        false
    }
}

/// Restores the enclosing synchronous evaluation, including during unwinding.
struct ReadScope {
    previous: Option<bool>,
}

impl ReadScope {
    fn enter() -> Self {
        Self {
            previous: SIGNAL_READ.replace(Some(false)),
        }
    }
}

impl Drop for ReadScope {
    fn drop(&mut self) {
        let read = SIGNAL_READ.replace(self.previous) == Some(true);
        if read {
            mark_signal_read();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{panic::catch_unwind, sync::Barrier};

    use super::*;

    #[test]
    fn nested_evaluations_propagate_reads_without_contaminating_siblings() {
        let outer = Expr::evaluate(
            || {
                let inner = Expr::evaluate(mark_signal_read, Js::source(""));
                assert!(!inner.is_static());
                let sibling = Expr::evaluate(|| 1, Js::source(""));
                assert!(sibling.is_static());
            },
            Js::source(""),
        );
        assert!(!outer.is_static());
        assert!(Expr::evaluate(|| 1, Js::source("")).is_static());
    }

    #[test]
    fn panic_restores_the_parent_scope_and_preserves_its_reads() {
        let outer = Expr::evaluate(
            || {
                let panic = catch_unwind(|| {
                    Expr::evaluate(
                        || {
                            mark_signal_read();
                            panic!("evaluation failed");
                        },
                        Js::source(""),
                    )
                });
                assert!(panic.is_err());
            },
            Js::source(""),
        );
        assert!(!outer.is_static());
        assert!(Expr::evaluate(|| 1, Js::source("")).is_static());
    }

    #[test]
    fn concurrent_evaluations_do_not_share_reads() {
        let barrier = Barrier::new(2);
        std::thread::scope(|scope| {
            let dynamic = scope.spawn(|| {
                Expr::evaluate(
                    || {
                        mark_signal_read();
                        barrier.wait();
                        barrier.wait();
                    },
                    Js::source(""),
                )
            });
            let constant = Expr::evaluate(
                || {
                    barrier.wait();
                    barrier.wait();
                },
                Js::source(""),
            );
            assert!(constant.is_static());
            assert!(!dynamic.join().unwrap().is_static());
        });
    }
}
