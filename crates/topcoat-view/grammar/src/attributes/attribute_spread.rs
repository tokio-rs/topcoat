use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::hir::{AttributeBuilder, LowerAttribute},
    template::TemplateExpr,
    view::hir::{ExprKind, LowerView, ViewBuilder},
};

/// A parenthesized expression that inserts whole attributes, like the
/// `(attrs)` in `<button (attrs)>`.
///
/// Unlike an [`Attribute`](super::Attribute), it has no `=value`. In an
/// opening tag, the expression must implement
/// [`AttributeViewParts`](topcoat_view::AttributeViewParts). In `attributes!`,
/// it extends the collection being built, so it must be iterable as
/// [`AttributeKey`](topcoat_view::AttributeKey) and
/// [`AttributeValue`](topcoat_view::AttributeValue) pairs, as an
/// [`Attributes`](topcoat_view::Attributes) collection is.
///
/// A parenthesized expression followed by `=` is an attribute name instead,
/// as in `(name)="value"`.
pub struct AttributeSpread {
    /// The inserted expression.
    pub expr: TemplateExpr,
}

impl LowerView for AttributeSpread {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.expr(ExprKind::Attributes, self.expr.expr.to_token_stream());
    }
}

impl LowerAttribute for AttributeSpread {
    fn lower(&self, builder: &mut AttributeBuilder) {
        // A spread contributes an unknown number of attributes, so it adds no
        // static capacity. It extends the collection being built, with its keys
        // replacing any already present.
        let expr = &self.expr.expr;
        builder.insert_block(0, quote! { __attrs.extend(#expr); });
    }
}

impl Parse for AttributeSpread {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr: input.parse()?,
        })
    }
}

impl ParseOption for AttributeSpread {
    fn peek(input: ParseStream) -> bool {
        if !input.peek(Paren) {
            return false;
        }
        // Distinguish a spread `(expr)` from a dynamic key `(name)=value` by
        // looking past the parenthesized group on a fork: a `=` means it is a
        // key, anything else (another attribute, `>`, end of input) a spread.
        let fork = input.fork();
        fork.parse::<TemplateExpr>().is_ok() && !fork.peek(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for AttributeSpread {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.expr.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs [`AttributeSpread::peek`] on `source`, draining the stream
    /// afterwards so the one-shot parser does not complain about the tokens
    /// `peek` deliberately leaves unconsumed.
    fn peek(source: &str) -> bool {
        syn::parse::Parser::parse_str(
            |input: ParseStream| {
                let peeked = AttributeSpread::peek(input);
                input.parse::<proc_macro2::TokenStream>()?;
                Ok(peeked)
            },
            source,
        )
        .unwrap()
    }

    #[test]
    fn parses_spread_expression() {
        let spread: AttributeSpread = syn::parse_str("(attrs)").unwrap();
        assert_eq!(spread.expr.expr.to_token_stream().to_string(), "attrs");
    }

    #[test]
    fn peeks_spread_without_equals() {
        assert!(peek("(attrs)"));
    }

    #[test]
    fn does_not_peek_dynamic_key() {
        // `(name)=value` is a dynamic attribute key, not a spread.
        assert!(!peek(r#"(name)="value""#));
    }
}
