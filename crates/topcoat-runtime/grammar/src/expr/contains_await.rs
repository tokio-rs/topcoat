use syn::visit::{self, Visit};
use syn::{ExprBlock, ExprIf};

#[derive(Default)]
pub(super) struct ContainsAwait {
    found: bool,
}
impl ContainsAwait {
    pub(super) fn in_if(if_expr: &ExprIf) -> bool {
        let mut visitor = Self::default();
        visitor.visit_expr_if(if_expr);
        visitor.found
    }

    pub(super) fn in_block(block_expr: &ExprBlock) -> bool {
        let mut visitor = Self::default();
        visitor.visit_expr_block(block_expr);
        visitor.found
    }
}

impl<'ats> Visit<'ats> for ContainsAwait {
    fn visit_expr_await(&mut self, node: &'ats syn::ExprAwait) {
        self.found = true;
        visit::visit_expr_await(self, node);
    }

    fn visit_expr_closure(&mut self, _node: &'ats syn::ExprClosure) {}
}
