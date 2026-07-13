use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::{AttributeWriter, WriteAttribute},
    template::TemplateExpr,
    view::{ExprKind, ViewWriter, WriteView},
};

/// A parenthesized expression spread into an element as a complete attribute
/// fragment, e.g. the `(attrs)` in `<button (attrs)>`.
///
/// Unlike an [`Attribute`](super::Attribute), a spread has no `=value`: the
/// expression evaluates to a value implementing
/// [`AttributeViewParts`](topcoat_view::AttributeViewParts) and contributes
/// zero or more whole attributes. A parenthesized expression *followed* by `=`
/// is instead a dynamic attribute key (`(name)="value"`), so spreads are only
/// recognized when no `=` follows.
pub struct AttributeSpread {
    pub expr: TemplateExpr,
}

impl WriteView for AttributeSpread {
    fn write(&self, writer: &mut ViewWriter) {
        writer.write_expr(ExprKind::Attributes, self.expr.expr.to_token_stream());
    }
}

impl WriteAttribute for AttributeSpread {
    fn write(&self, writer: &mut AttributeWriter) {
        // A spread contributes an unknown number of attributes, so it adds no
        // static capacity. It extends the collection being built, with its keys
        // replacing any already present.
        let expr = &self.expr.expr;
        writer.insert_block(0, quote! { __attrs.extend(#expr); });
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
