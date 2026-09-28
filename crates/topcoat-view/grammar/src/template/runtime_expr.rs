use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Token, parenthesized,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_runtime_macro};

use crate::view::hir::{ExprKind, LowerView, ViewBuilder};

/// A `$(`...`)` runtime expression, lowered through `runtime::expr!`.
#[derive(Debug, Clone)]
pub struct RuntimeExpr {
    pub dollar: Token![$],
    pub paren: syn::token::Paren,
    pub expr: TokenStream,
}

impl PartialEq for RuntimeExpr {
    fn eq(&self, other: &Self) -> bool {
        self.dollar == other.dollar
            && self.paren == other.paren
            && syn::Expr::Verbatim(self.expr.clone()) == syn::Expr::Verbatim(other.expr.clone())
    }
}

impl LowerView for RuntimeExpr {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.expr(ExprKind::Node, self.to_token_stream());
    }
}

impl Parse for RuntimeExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            dollar: input.parse()?,
            paren: parenthesized!(content in input),
            expr: content.parse()?,
        })
    }
}

impl ParseOption for RuntimeExpr {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![$])
    }
}

impl ToTokens for RuntimeExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        quote! {
            #topcoat_runtime_macro::expr! { #expr }
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for RuntimeExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        "$".pretty_print(printer);
        "(".pretty_print(printer);
        syn::parse2::<syn::Expr>(self.expr.clone())
            .unwrap_or_else(|_| syn::Expr::Verbatim(self.expr.clone()))
            .pretty_print(printer);
        ")".pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::{ToTokens, quote};

    use super::*;

    #[test]
    fn tokens_wrap_expression() {
        let expr = syn::parse_str::<RuntimeExpr>("$(value + 1)").unwrap();
        assert_eq!(
            expr.to_token_stream().to_string(),
            quote! { #topcoat_runtime_macro::expr! { value + 1 } }.to_string(),
        );
    }

    #[test]
    fn forwards_unparsed_tokens() {
        for input in [quote! { |e: Event| e. }, quote! {}, quote! { value, }] {
            let expr = syn::parse2::<RuntimeExpr>(quote! { $(#input) }).unwrap();
            assert_eq!(
                expr.to_token_stream().to_string(),
                quote! { #topcoat_runtime_macro::expr! { #input } }.to_string(),
            );
        }
    }

    #[test]
    fn view_accepts_incomplete_runtime_expression() {
        let view = syn::parse_str::<crate::view::View>("<input @input=$(|e: Event| e.)>")
            .unwrap();
        let expanded = view.to_token_stream().to_string();
        let runtime = quote! { #topcoat_runtime_macro::expr! { |e: Event| e. } }.to_string();
        assert!(expanded.contains(&runtime));
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn formats_runtime_tokens() {
        use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

        let mut registry = Registry::new();
        registry.register_macro::<crate::view::View>("view");
        for (input, expected) in [
            ("|e:Event|e.target", "|e: Event| e.target"),
            ("|e: Event| e.", "|e: Event| e."),
        ] {
            let source = format!("view! {{ <input @input=$({input})> }}");
            let formatted = pretty_print_str(&registry, &source).unwrap();
            assert!(formatted.contains(&format!("$({expected})")));
            assert_eq!(pretty_print_str(&registry, &formatted).unwrap(), formatted);
        }
    }
}
