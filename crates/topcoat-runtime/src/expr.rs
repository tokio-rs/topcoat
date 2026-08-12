use topcoat_core::context::Cx;
use topcoat_view::{Markup, NodeViewParts, PartsWriter};

#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: Markup,
}

impl<T> Expr<T> {
    #[inline]
    pub fn new(evaluated: T, js: Markup) -> Self {
        Self { evaluated, js }
    }

    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, Markup) {
        (self.evaluated, self.js)
    }
}

impl<T> NodeViewParts for Expr<T>
where
    T: NodeViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str_unescaped(&"<!-- ::topcoat::expr::start(\"");
        topcoat_view::internal::view(parts, self.js);
        parts.push_promoted_str_unescaped(&"\") -->");
        self.evaluated.into_view_parts(cx, parts);
        parts.push_promoted_str_unescaped(&"<!-- ::topcoat::expr::end -->");
    }
}
