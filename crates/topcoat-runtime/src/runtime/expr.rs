use topcoat_view::runtime::{NodeViewParts, Unescaped, ViewPart, ViewParts};

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
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::expr::start(\""));
        parts.push(self.js);
        parts.push(Unescaped::new_unchecked("\") -->"));
        self.evaluated.into_view_parts(parts);
        parts.push(Unescaped::new_unchecked("<!-- ::topcoat::expr::end -->"));
    }
}
