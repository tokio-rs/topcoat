use syn::{
    ExprBlock, ExprIf, Macro,
    visit::{self, Visit},
};

use super::builtin_macro::BuiltinMacro;

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

    /// A `raw!` whose JavaScript awaits makes its block await too.
    fn visit_macro(&mut self, node: &'ats Macro) {
        if BuiltinMacro::parse(node).is_ok_and(|mac| mac.awaits()) {
            self.found = true;
        }
    }

    fn visit_expr_closure(&mut self, _node: &'ats syn::ExprClosure) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `raw!` awaits, as an expression or a statement, only when `await` is
    /// a word of its own in its JavaScript.
    #[test]
    fn raw_javascript_that_awaits_makes_its_block_await() {
        let awaits: ExprBlock = syn::parse_quote!({
            let x = raw!("await ${y}", 1);
        });
        assert!(ContainsAwait::in_block(&awaits));
        let statement: ExprBlock = syn::parse_quote!({
            raw!("await done()");
        });
        assert!(ContainsAwait::in_block(&statement));
        let named: ExprBlock = syn::parse_quote!({
            let x = raw!("awaited(${y})", 1);
        });
        assert!(!ContainsAwait::in_block(&named));
        let nested: ExprBlock = syn::parse_quote!({
            let f = || raw!("await ${y}", 1);
        });
        assert!(!ContainsAwait::in_block(&nested));
    }
}
