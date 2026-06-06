use std::fmt::{self, Display};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    template::TemplateExpr,
    view::{ExprKind, HtmlIdent, ViewWriter, WriteView},
};

/// The name part of a single `name=value` attribute on an
/// [`Element`](super::Element) or [`Component`](super::Component). Either an
/// HTML identifier (`data-foo`, `aria-label`) or a parenthesized Rust
/// expression that resolves to the attribute name at runtime.
pub enum AttributeKey {
    Ident(HtmlIdent),
    Expr(Box<TemplateExpr>),
}

impl WriteView for AttributeKey {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Ident(inner) => writer.write_str_unescaped(&inner.to_string()),
            Self::Expr(inner) => {
                writer.write_expr(ExprKind::AttributeKey, inner.expr.to_token_stream())
            }
        }
    }
}

impl Parse for AttributeKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) {
            Ok(Self::Ident(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ParseOption for AttributeKey {
    fn peek(input: ParseStream) -> bool {
        TemplateExpr::peek(input) || HtmlIdent::peek(input)
    }
}

impl ToTokens for AttributeKey {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(inner) => inner.to_string().to_tokens(tokens),
            Self::Expr(inner) => inner.expr.to_tokens(tokens),
        }
    }
}

impl Display for AttributeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(inner) => Display::fmt(inner, f),
            Self::Expr(_) => f.write_str("<expr>"),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for AttributeKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Ident(inner) => inner.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}
