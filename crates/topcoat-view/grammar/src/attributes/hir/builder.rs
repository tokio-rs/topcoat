use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Pat};

use super::{MatchArm, Node, Scope};

/// AST nodes that can lower themselves into an [`AttributeBuilder`].
pub(crate) trait LowerAttribute {
    fn lower(&self, builder: &mut AttributeBuilder);
}

/// Lowers an [`Attributes`](crate::attributes::Attributes) AST into a
/// [`Scope`], the HIR the expansion is emitted from.
///
/// Each `__attrs.insert(...)` call is recorded as a [`Node::Insert`] along
/// with how many entries it contributes; control-flow nodes (`if`/`for`/
/// `match`) recurse into nested builders. The capacity hint passed to
/// `Attributes::with_capacity` is derived from these recorded contributions.
pub(crate) struct AttributeBuilder {
    nodes: Vec<Node>,
}

impl AttributeBuilder {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Records a single `__attrs.insert(key, value);` call.
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert(&mut self, key: TokenStream, value: TokenStream) {
        self.nodes.push(Node::Insert {
            tokens: quote! { __attrs.insert(__cx, #key, #value); },
            capacity: 1,
        });
    }

    /// Records a self-contained block that performs `capacity` inserts into
    /// `__attrs`.
    pub fn insert_block(&mut self, capacity: usize, tokens: TokenStream) {
        self.nodes.push(Node::Insert { tokens, capacity });
    }

    pub fn local_binding(&mut self, pat: &Pat, expr: &Expr) {
        self.nodes.push(Node::Local {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
        });
    }

    pub fn statement(&mut self, tokens: TokenStream) {
        self.nodes.push(Node::Statement { tokens });
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut AttributeBuilder)) {
        let mut body = AttributeBuilder::new();
        f(&mut body);
        self.nodes.push(Node::For {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: body.finish(),
        });
    }

    pub fn if_else(
        &mut self,
        cond: &Expr,
        f: impl FnOnce(&mut AttributeBuilder, &mut AttributeBuilder),
    ) {
        let mut then_branch = AttributeBuilder::new();
        let mut else_branch = AttributeBuilder::new();
        f(&mut then_branch, &mut else_branch);
        self.nodes.push(Node::If {
            cond: cond.clone(),
            then_branch: then_branch.finish(),
            else_branch: else_branch.finish(),
        });
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        let mut builder = MatchArmsBuilder { arms: Vec::new() };
        f(&mut builder);
        self.nodes.push(Node::Match {
            expr: Box::new(expr.clone()),
            arms: builder.arms,
        });
    }

    /// Returns the lowered [`Scope`].
    pub fn finish(self) -> Scope {
        Scope::new(self.nodes)
    }
}

/// Collects the arms of a [`Node::Match`], each lowered into its own
/// [`Scope`].
pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut AttributeBuilder)) {
        let mut body = AttributeBuilder::new();
        f(&mut body);
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: body.finish(),
        });
    }
}
