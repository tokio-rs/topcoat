use std::cell::Cell;

use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter};

use crate::{Js, Surrogate, Surrogated};

/// A value with a JavaScript expression that can produce it in the browser.
///
/// Expressions that read no signals render as static content. Their
/// JavaScript remains available when needed for browser execution.
#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: Js,
    pub(crate) is_static: bool,
    pub(crate) synced: bool,
}

impl<T> Expr<T> {
    /// Evaluates an expression while observing signal reads.
    ///
    /// The evaluation is synchronous. Its signal reads must represent the
    /// dependencies of the JavaScript expression as well.
    #[inline]
    pub fn evaluate(evaluate: impl FnOnce() -> T, js: Js) -> Self {
        let _scope = ReadScope::enter();
        let evaluated = evaluate();
        Self {
            evaluated,
            js,
            is_static: SIGNAL_READ.get() == Some(false),
            synced: false,
        }
    }

    /// Whether this expression is known not to require reactive evaluation.
    ///
    /// Signal reads during [`evaluate`](Self::evaluate) make it dynamic.
    #[inline]
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Makes this expression dynamic even if it reads no signals.
    ///
    /// A static expression only renders its value. A dynamic one is also
    /// evaluated in the browser whenever its content renders. As a bind
    /// attribute, this sets the element's live state on every render, such as
    /// the value of an input the user has edited, instead of only its default.
    ///
    /// ```rust
    /// use topcoat::{Result, context::Cx, runtime::Expr, view::*};
    ///
    /// #[component]
    /// async fn search(cx: &Cx) -> Result<impl View> {
    ///     // Every render clears what the user typed.
    ///     let query = Expr::from(String::new()).dynamic();
    ///
    ///     Ok(view! { <input :value=(query)> })
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn dynamic(mut self) -> Self {
        self.is_static = false;
        self
    }

    /// Makes the browser value win over the server's on the first render.
    ///
    /// The server renders an expression's value into the HTML. The browser
    /// evaluates the expression again, but the first evaluation only
    /// subscribes: the runtime keeps the server's text, because the browser
    /// is expected to compute the same value. A synced expression says the
    /// browser value is the authoritative one, so the first evaluation
    /// writes it.
    ///
    /// Use it when the server cannot compute the value the browser should
    /// show, such as the visitor's language. Being synced implies
    /// [`dynamic`](Self::dynamic). An attribute binding writes on every
    /// evaluation, so the flag changes text expressions only.
    ///
    /// ```rust
    /// use topcoat::{Result, context::Cx, runtime::expr, view::*};
    ///
    /// #[component]
    /// async fn greeting(cx: &Cx) -> Result<impl View> {
    ///     // The server cannot know the visitor's language; the browser can.
    ///     let language = expr!(raw!("navigator.language", "en")).synced();
    ///
    ///     Ok(view! { <p>(language)</p> })
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn synced(mut self) -> Self {
        self.is_static = false;
        self.synced = true;
        self
    }

    /// Returns the value and its JavaScript, including for static expressions.
    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, Js) {
        (self.evaluated, self.js)
    }

    /// Returns the captured value, carrying its dependencies into evaluation.
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
            comment.push_promoted_str_unescaped(&"\"");
            if self.synced {
                comment.push_promoted_str_unescaped(&", true");
            }
            comment.push_promoted_str_unescaped(&")");
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
