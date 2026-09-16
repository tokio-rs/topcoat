use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter};

use crate::{Js, Surrogate, Surrogated};

/// A value with a JavaScript expression that can produce it in the browser.
///
/// Converting an ordinary value with [`From`] creates a static expression.
/// Static expressions render without browser bindings or marker comments.
#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: Js,
    pub(crate) is_static: bool,
}

impl<T> Expr<T> {
    #[inline]
    pub fn new(evaluated: T, js: Js) -> Self {
        Self {
            evaluated,
            js,
            is_static: false,
        }
    }

    /// Whether this expression was converted from an ordinary value.
    ///
    /// Expressions created with [`new`](Self::new) are dynamic, even when
    /// their JavaScript evaluates to a constant.
    #[inline]
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Returns the value and its JavaScript, including for static expressions.
    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, Js) {
        (self.evaluated, self.js)
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
        Self {
            evaluated: surrogate.into_real(),
            js,
            is_static: true,
        }
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
