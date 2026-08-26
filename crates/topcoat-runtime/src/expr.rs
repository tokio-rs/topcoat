use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter};

use crate::Js;

#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: Js,
}

impl<T> Expr<T> {
    #[inline]
    pub fn new(evaluated: T, js: Js) -> Self {
        Self { evaluated, js }
    }

    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, Js) {
        (self.evaluated, self.js)
    }
}

impl<T> NodeViewParts for Expr<T>
where
    T: NodeViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
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
