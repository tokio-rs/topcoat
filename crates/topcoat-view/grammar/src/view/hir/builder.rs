use proc_macro2::{Span, TokenStream};
use syn::{Expr, Pat, Path};

use super::{
    Component, ExprKind, ExprNode, ForLoop, IfElse, Local, MatchArm, MatchExpr, Node, Scope,
    Statement, StaticSegment,
};
use crate::view::{NamedArg, Nodes};

/// AST nodes that can lower themselves into a [`ViewBuilder`].
pub(crate) trait LowerView {
    fn lower(&self, builder: &mut ViewBuilder);
}

/// Lowers the AST of a `view!` invocation into a [`Scope`], the HIR the
/// expansion is emitted from.
///
/// Adjacent literal markup is concatenated into `static_segment` and flushed
/// as a single [`Node::StaticSegment`] whenever a dynamic node (expression,
/// control flow) is lowered.
pub(crate) struct ViewBuilder {
    nodes: Vec<Node>,
    static_segment: String,
}

impl ViewBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            static_segment: String::new(),
        }
    }

    fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let mut static_segment = String::new();
            std::mem::swap(&mut self.static_segment, &mut static_segment);
            self.nodes.push(Node::StaticSegment(StaticSegment {
                string: static_segment,
            }));
        }
    }
    fn write_in_context(&mut self, context: topcoat_view::HtmlContext, s: &str) {
        let mut f = topcoat_view::Formatter::new(&mut self.static_segment);
        context.writer(&mut f).write_str(s);
    }

    pub fn str_unescaped(&mut self, s: &str) {
        self.static_segment.push_str(s);
    }

    /// Appends literal text escaped for a text node position.
    pub fn text(&mut self, s: &str) {
        self.write_in_context(topcoat_view::HtmlContext::Text, s);
    }

    /// Appends literal text escaped for a double-quoted attribute value
    /// position.
    pub fn attribute_value(&mut self, s: &str) {
        self.write_in_context(topcoat_view::HtmlContext::AttributeValue, s);
    }

    pub fn expr(&mut self, kind: ExprKind, tokens: TokenStream) {
        self.flush();
        self.nodes.push(Node::ExprNode(ExprNode { kind, tokens }));
    }

    pub fn local_binding(&mut self, pat: &Pat, expr: &Expr) {
        // Locals do not need flush because they don't affect static segments.
        self.nodes.push(Node::Local(Local {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
        }));
    }

    pub fn statement(&mut self, tokens: TokenStream) {
        self.flush();
        self.nodes.push(Node::Statement(Statement { tokens }));
    }

    /// Lowers a component invocation, keeping the path, named arguments, and
    /// lowered children for emission by [`Scope`].
    pub fn component(
        &mut self,
        path: &Path,
        named_args: &[NamedArg],
        children: &Nodes,
        span: Span,
    ) {
        self.flush();
        let children = (!children.is_empty()).then(|| {
            let mut child_builder = ViewBuilder::new();
            children.lower(&mut child_builder);
            child_builder.finish()
        });
        self.nodes.push(Node::Component(Component {
            path: path.clone(),
            named_args: named_args.to_vec(),
            children,
            span,
        }));
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut ViewBuilder)) {
        self.flush();
        let mut body = ViewBuilder::new();
        f(&mut body);
        self.nodes.push(Node::ForLoop(ForLoop {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: body.finish(),
        }));
    }

    pub fn if_else(&mut self, expr: &Expr, f: impl FnOnce(&mut ViewBuilder, &mut ViewBuilder)) {
        self.flush();
        let mut then_branch = ViewBuilder::new();
        let mut else_branch = ViewBuilder::new();
        f(&mut then_branch, &mut else_branch);
        self.nodes.push(Node::IfElse(IfElse {
            expr: expr.clone(),
            then_branch: then_branch.finish(),
            else_branch: else_branch.finish(),
        }));
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        self.flush();
        let mut builder = MatchArmsBuilder { arms: Vec::new() };
        f(&mut builder);
        self.nodes.push(Node::MatchExpr(MatchExpr {
            expr: Box::new(expr.clone()),
            arms: builder.arms,
        }));
    }

    /// Flushes any pending literal markup and returns the lowered [`Scope`].
    pub fn finish(mut self) -> Scope {
        self.flush();
        Scope::new(self.nodes)
    }
}

/// Collects the arms of a [`Node::MatchExpr`], each lowered into its own
/// [`Scope`].
pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut ViewBuilder)) {
        let mut body = ViewBuilder::new();
        f(&mut body);
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: body.finish(),
        });
    }
}
