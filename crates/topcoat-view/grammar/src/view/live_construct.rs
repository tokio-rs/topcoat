use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::{Brace, Paren},
};

use crate::{
    template::{TemplateBlock, TemplateMatchArm},
    view::{
        Component, Node, Nodes,
        hir::{LiveScrutineeInput, LowerView, ViewBuilder},
    },
};

pub(crate) mod kw {
    syn::custom_keyword!(live);
}

/// A `live match expr { ... }` construct: a match that consumes a reactive
/// expression and re-renders its arms in place when the state changes.
pub struct LiveMatch {
    pub live_token: kw::live,
    pub match_token: Token![match],
    pub expr: LiveExpr,
    pub brace_token: Brace,
    pub arms: Vec<TemplateMatchArm<Node>>,
}

impl Parse for LiveMatch {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            live_token: input.parse()?,
            match_token: input.parse()?,
            expr: input.parse()?,
            brace_token: syn::braced!(content in input),
            arms: {
                let mut arms = Vec::new();
                while !content.is_empty() {
                    arms.push(content.parse()?);
                }
                arms
            },
        })
    }
}

impl LowerView for LiveMatch {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.live_node(
            self.expr.scrutinee_input(),
            self.live_token.span(),
            |arms| {
                for arm in &self.arms {
                    arm.lower(arms);
                }
            },
        );
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveMatch {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::{BreakMode, Delim};

        "live ".pretty_print(printer);
        self.match_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);

        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                for (index, arm) in self.arms.iter().enumerate() {
                    arm.pretty_print(printer);
                    if index < self.arms.len() - 1 {
                        printer.scan_force_break();
                        printer.scan_break();
                    }
                }
            });
    }
}

/// A `live if let pat = expr { ... }` construct, with an optional `else`
/// block for the states the pattern misses.
pub struct LiveIfLet {
    pub live_token: kw::live,
    pub if_token: Token![if],
    pub let_token: Token![let],
    pub pat: Pat,
    pub eq_token: Token![=],
    pub expr: LiveExpr,
    pub then_branch: TemplateBlock<Nodes>,
    pub else_branch: Option<LiveElse>,
}

impl Parse for LiveIfLet {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            live_token: input.parse()?,
            if_token: input.parse()?,
            let_token: input.parse()?,
            pat: Pat::parse_multi_with_leading_vert(input)?,
            eq_token: input.parse()?,
            expr: input.parse()?,
            then_branch: input.parse()?,
            else_branch: {
                if input.peek(Token![else]) {
                    Some(input.parse()?)
                } else {
                    None
                }
            },
        })
    }
}

impl LowerView for LiveIfLet {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.live_node(
            self.expr.scrutinee_input(),
            self.live_token.span(),
            |arms| {
                arms.arm(&self.pat, None, |body| self.then_branch.lower(body));
                let wildcard: Pat = syn::parse_quote!(_);
                arms.arm(&wildcard, None, |body| {
                    if let Some(else_branch) = &self.else_branch {
                        else_branch.body.lower(body);
                    }
                });
            },
        );
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveIfLet {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        "live ".pretty_print(printer);
        self.if_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.let_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.then_branch.pretty_print(printer);
        if let Some(else_branch) = &self.else_branch {
            " ".pretty_print(printer);
            else_branch.else_token.pretty_print(printer);
            " ".pretty_print(printer);
            else_branch.body.pretty_print(printer);
        }
    }
}

/// The trailing `else { ... }` of a [`LiveIfLet`].
pub struct LiveElse {
    pub else_token: Token![else],
    pub body: TemplateBlock<Nodes>,
}

impl Parse for LiveElse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            else_token: input.parse()?,
            body: input.parse()?,
        })
    }
}

/// The reactive expression a `live` construct consumes: a component
/// invocation adopted as a handle, or a plain expression such as a
/// `defer(...)` call or a bound reactive value.
pub enum LiveExpr {
    Component(Component),
    Expr(Box<Expr>),
}

impl LiveExpr {
    /// Classifies the scrutinee for lowering.
    ///
    /// A call of the form `defer(future)` is the deferred load, and the
    /// expansion supplies its context argument. Other call shapes parsed as
    /// invocations adopt the component. Everything else is an expression
    /// that must already be a reactive value.
    pub(crate) fn scrutinee_input(&self) -> LiveScrutineeInput<'_> {
        match self {
            Self::Component(component) => LiveScrutineeInput::Component {
                path: &component.path,
                named_args: component.props(),
                key: component.identity_key(),
                children: &component.children,
                span: component.paren_token.span.span(),
            },
            Self::Expr(expr) => match defer_argument(expr) {
                Some(future) => LiveScrutineeInput::Defer(future),
                None => LiveScrutineeInput::Expr(expr),
            },
        }
    }
}

/// Returns the single argument of a `defer(...)` call, `None` for any other
/// expression.
fn defer_argument(expr: &Expr) -> Option<&Expr> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    let is_defer = path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "defer");
    if is_defer && call.args.len() == 1 {
        call.args.first()
    } else {
        None
    }
}

/// Whether the input starts a `defer(...)` call, which parses as an
/// expression even though its shape matches a component invocation.
fn peek_defer(input: ParseStream) -> bool {
    let fork = input.fork();
    let Ok(path) = fork.parse::<syn::Path>() else {
        return false;
    };
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "defer")
        && fork.peek(Paren)
}

impl Parse for LiveExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // A component invocation is `path(...)`; `defer(...)` matches that
        // shape but is the deferred load, parsed as an expression. Anything
        // else, like a bound value or a method call, is an expression too.
        if !peek_defer(input) {
            let fork = input.fork();
            if fork.parse::<syn::Path>().is_ok() && fork.peek(Paren) {
                return Ok(Self::Component(input.parse()?));
            }
        }
        Ok(Self::Expr(Box::new(
            input.call(Expr::parse_without_eager_brace)?,
        )))
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Component(inner) => inner.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}
