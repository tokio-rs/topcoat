use topcoat_core::context::Cx;
use topcoat_view::{NodeViewParts, PartsWriter, ViewPart};

#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub(crate) evaluated: T,
    pub(crate) js: ViewPart,
}

impl<T> Expr<T> {
    #[inline]
    pub fn new(evaluated: T, js: ViewPart) -> Self {
        Self { evaluated, js }
    }

    #[inline]
    pub fn into_evaluated_and_js(self) -> (T, ViewPart) {
        (self.evaluated, self.js)
    }
}

impl<T> NodeViewParts for Expr<T>
where
    T: NodeViewParts,
{
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str_unescaped("<!-- ::topcoat::expr::start(\"");
        parts.push_part(self.js);
        parts.push_str_unescaped("\") -->");
        self.evaluated.into_view_parts(cx, parts);
        parts.push_str_unescaped("<!-- ::topcoat::expr::end -->");
    }
}
