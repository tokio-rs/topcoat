use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
};

use crate::{ast::parse_option::ParseOption, output::ViewWriter};

/// A parenthesized Rust expression embedded as a child node, e.g. `(name)` or
/// `(slot.await)`. The value is rendered through
/// [`Fragment`](crate::runtime::Fragment) (i.e. escaped by default).
pub struct NodeExpr {
    pub paren: syn::token::Paren,
    pub expr: syn::Expr,
}

impl NodeExpr {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let expr = &self.expr;
        writer.write_expr(expr.to_token_stream());
    }
}

impl Parse for NodeExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren: parenthesized!(content in input),
            expr: content.parse()?,
        })
    }
}

impl ParseOption for NodeExpr {
    fn peek(input: ParseStream) -> bool {
        input.peek(syn::token::Paren)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeExpr {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        "(".pretty_print(printer);
        self.expr.pretty_print(printer);
        ")".pretty_print(printer);
    }
}
