use std::{cell::Cell, rc::Rc};

use proc_macro2::{Span, TokenStream};
use syn::{Expr, Pat, Path};

use super::{
    Component, ExprKind, ExprNode, ForLoop, IfElse, LiveNode, LiveScrutinee, Local, MatchArm,
    MatchExpr, Node, Scope, Statement, StaticSegment,
};
use crate::view::{NamedArg, NamedArgValue, Nodes};

/// Whether a macro path is a `view!` invocation, by its final segment.
///
/// The path cannot be resolved at expansion time, so any macro named `view`
/// is taken to be the view macro, however the caller reaches it.
pub(crate) fn is_view_macro(path: &Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "view")
}

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
    /// Numbers reactive nodes in lowering order, shared like `sites`, so
    /// each frame's prelude names its nodes' slots and tickets uniquely.
    live_nodes: Rc<Cell<u32>>,
    /// Whether this builder lowers a `for` body, where every invocation
    /// site repeats.
    repeats: bool,
}

impl ViewBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            static_segment: String::new(),
            sites: Rc::new(Cell::new(0)),
            live_nodes: Rc::new(Cell::new(0)),
            repeats: false,
        }
    }

    /// Returns a builder for a nested scope, sharing this builder's site
    /// numbering.
    fn nested(&self, repeats: bool) -> Self {
        Self {
            nodes: Vec::new(),
            static_segment: String::new(),
            sites: Rc::clone(&self.sites),
            live_nodes: Rc::clone(&self.live_nodes),
            repeats,
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
        let component = self.lower_component(path, named_args, key, children, span);
        self.nodes.push(Node::Component(component));
    }

    fn lower_component(
        &mut self,
        path: &Path,
        named_args: Vec<NamedArg>,
        key: Option<&NamedArg>,
        children: &Nodes,
        span: Span,
    ) -> Component {
        let ordinal = self.sites.get();
        self.sites.set(ordinal + 1);
        let children = (!children.is_empty()).then(|| {
            let mut child_builder = self.nested(false);
            children.lower(&mut child_builder);
            child_builder.finish()
        });
        // A `view!` block in argument position is a view-valued argument:
        // lowered here so its own invocations number within this expansion,
        // and adopted as a handle by the live emission.
        let arg_views = named_args
            .iter()
            .map(|arg| match &arg.value {
                NamedArgValue::Expr(Expr::Macro(mac)) if is_view_macro(&mac.mac.path) => Some(
                    match syn::parse2::<crate::view::View>(mac.mac.tokens.clone()) {
                        Ok(view) => {
                            let mut view_builder = self.nested(self.repeats);
                            view.nodes.lower(&mut view_builder);
                            Ok(view_builder.finish())
                        }
                        Err(error) => Err(error),
                    },
                ),
                _ => None,
            })
            .collect();
        Component {
            path: path.clone(),
            named_args,
            arg_views,
            key: key.cloned(),
            ordinal,
            repeats: self.repeats,
            children,
            span,
        }
    }

    /// Lowers a `live` construct into a reactive node, numbering it and
    /// lowering each arm into a scope of its own.
    pub fn live_node(
        &mut self,
        scrutinee: LiveScrutineeInput<'_>,
        span: Span,
        f: impl FnOnce(&mut MatchArmsBuilder),
    ) {
        self.flush();
        let ordinal = self.live_nodes.get();
        self.live_nodes.set(ordinal + 1);
        let scrutinee = match scrutinee {
            LiveScrutineeInput::Defer(future) => LiveScrutinee::Defer(Box::new(future.clone())),
            LiveScrutineeInput::Component {
                path,
                named_args,
                key,
                children,
                span,
            } => LiveScrutinee::Component(Box::new(
                self.lower_component(path, named_args, key, children, span),
            )),
            LiveScrutineeInput::Expr(expr) => LiveScrutinee::Expr(Box::new(expr.clone())),
        };
        let mut builder = MatchArmsBuilder {
            arms: Vec::new(),
            template: self.nested(self.repeats),
        };
        f(&mut builder);
        self.nodes.push(Node::Live(LiveNode {
            ordinal,
            scrutinee,
            arms: builder.arms,
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

/// The reactive expression a `live` construct consumes, as the AST hands it
/// to [`ViewBuilder::live_node`].
pub(crate) enum LiveScrutineeInput<'a> {
    /// A `defer(future)` call; the expansion supplies the context argument.
    Defer(&'a Expr),
    /// A component invocation, adopted so its states arrive at the node.
    Component {
        path: &'a Path,
        named_args: Vec<NamedArg>,
        key: Option<&'a NamedArg>,
        children: &'a Nodes,
        span: Span,
    },
    /// Any other expression, already a reactive value.
    Expr(&'a Expr),
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
