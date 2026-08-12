use std::{cell::Cell, rc::Rc};

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
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
    /// Numbers component invocation sites in lowering order, shared across
    /// the nested builders of one expansion so every site gets a distinct
    /// ordinal.
    sites: Rc<Cell<u32>>,
    /// Whether this builder lowers a `for` body, where every invocation
    /// site repeats.
    repeats: bool,
    /// Identifiers bound to view tokens in the enclosing function, so a bare
    /// interpolation of one lowers to a placement.
    tokens: Rc<std::collections::HashSet<String>>,
}

impl ViewBuilder {
    pub fn new() -> Self {
        Self::with_tokens(std::collections::HashSet::new())
    }

    /// Returns a builder that lowers bare interpolations of the named
    /// identifiers as view token placements.
    pub fn with_tokens(tokens: std::collections::HashSet<String>) -> Self {
        Self {
            nodes: Vec::new(),
            static_segment: String::new(),
            sites: Rc::new(Cell::new(0)),
            repeats: false,
            tokens: Rc::new(tokens),
        }
    }

    /// Returns a builder for a nested scope, sharing this builder's site
    /// numbering.
    fn nested(&self, repeats: bool) -> Self {
        Self {
            nodes: Vec::new(),
            static_segment: String::new(),
            sites: Rc::clone(&self.sites),
            repeats,
            tokens: Rc::clone(&self.tokens),
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
        if matches!(kind, ExprKind::Node) {
            // Interpolating a view token with `?` is a placement: the token's
            // view renders here and its error propagates through the render.
            if let Ok(syn::Expr::Try(try_expr)) = syn::parse2::<syn::Expr>(tokens.clone())
                && let syn::Expr::Path(path) = &*try_expr.expr
                && let Some(ident) = path.path.get_ident()
                && self.tokens.contains(&ident.to_string())
            {
                self.nodes
                    .push(Node::Place(try_expr.expr.to_token_stream()));
                return;
            }
            // A bare token hides that placement can fail, so require the `?`.
            if let Ok(ident) = syn::parse2::<syn::Ident>(tokens.clone())
                && self.tokens.contains(&ident.to_string())
            {
                let message = format!(
                    "placing child content propagates its error: write `({ident}?)`, \
                     or claim the error first with `{ident}.take_error()`"
                );
                self.nodes.push(Node::Statement(Statement {
                    tokens: quote::quote_spanned! { ident.span() =>
                        compile_error!(#message);
                    },
                }));
                return;
            }
        }
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
    ///
    /// The invocation is numbered with the next site ordinal, and remembers
    /// whether it sits in a `for` body, where it repeats without a `key`
    /// telling the repetitions apart. The children lower as their own
    /// invocations below this one, so they do not repeat relative to it.
    pub fn component(
        &mut self,
        path: &Path,
        named_args: Vec<NamedArg>,
        key: Option<&NamedArg>,
        children: &Nodes,
        span: Span,
    ) {
        self.flush();
        let ordinal = self.sites.get();
        self.sites.set(ordinal + 1);
        let children = (!children.is_empty()).then(|| {
            let mut child_builder = self.nested(false);
            children.lower(&mut child_builder);
            child_builder.finish()
        });
        self.nodes.push(Node::Component(Component {
            path: path.clone(),
            named_args,
            key: key.cloned(),
            ordinal,
            repeats: self.repeats,
            children,
            span,
        }));
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut ViewBuilder)) {
        self.flush();
        let mut body = self.nested(true);
        f(&mut body);
        self.nodes.push(Node::ForLoop(ForLoop {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: body.finish(),
        }));
    }

    pub fn if_else(&mut self, expr: &Expr, f: impl FnOnce(&mut ViewBuilder, &mut ViewBuilder)) {
        self.flush();
        let mut then_branch = self.nested(self.repeats);
        let mut else_branch = self.nested(self.repeats);
        f(&mut then_branch, &mut else_branch);
        self.nodes.push(Node::IfElse(IfElse {
            expr: expr.clone(),
            then_branch: then_branch.finish(),
            else_branch: else_branch.finish(),
        }));
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        self.flush();
        let mut builder = MatchArmsBuilder {
            arms: Vec::new(),
            template: self.nested(self.repeats),
        };
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
    /// An empty builder in the enclosing builder's context, cloned as the
    /// starting point of every arm body.
    template: ViewBuilder,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut ViewBuilder)) {
        let mut body = self.template.nested(self.template.repeats);
        f(&mut body);
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: body.finish(),
        });
    }
}
