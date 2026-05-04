use quote::quote;
use syn::{
    Expr, ExprBreak, ExprContinue, Pat, Token,
    parse::{Parse, ParseStream},
};

use crate::{
    ast::{NodeBlock, parse_option::ParseOption},
    output::ViewWriter,
};

/// A `for pat in expr { ... }` loop in view-body position. The body is
/// rendered once per iteration.
pub struct NodeForLoop {
    pub for_token: Token![for],
    pub pat: Box<Pat>,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: NodeBlock,
}

impl NodeForLoop {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let mut writer = writer.begin_for_loop(&self.pat, &self.expr);
        self.body.write(&mut writer);
    }
}

impl Parse for NodeForLoop {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            for_token: input.parse()?,
            pat: Box::new(input.call(Pat::parse_single)?),
            in_token: input.parse()?,
            expr: Box::new(input.call(Expr::parse_without_eager_brace)?),
            body: input.parse()?,
        })
    }
}

impl ParseOption for NodeForLoop {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![for])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeForLoop {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.for_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.in_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

/// A `continue;` statement. Parsed for completeness but currently rejected.
pub struct NodeContinue {
    pub expr_continue: ExprContinue,
    pub semi_token: Token![;],
}

impl NodeContinue {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let expr_continue = &self.expr_continue;
        let semi_token = &self.semi_token;
        writer.write_raw(quote! { #expr_continue #semi_token });
    }
}

impl Parse for NodeContinue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_continue: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for NodeContinue {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![continue])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeContinue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.semi_token.pretty_print(printer);
        todo!();
    }
}

/// A `break;` statement. Parsed for completeness but currently rejected.
pub struct NodeBreak {
    pub expr_break: ExprBreak,
    pub semi_token: Token![;],
}

impl NodeBreak {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let expr_break = &self.expr_break;
        let semi_token = &self.semi_token;
        writer.write_raw(quote! { #expr_break #semi_token });
    }
}

impl Parse for NodeBreak {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_break: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for NodeBreak {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![break])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeBreak {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.semi_token.pretty_print(printer);
        todo!();
    }
}
