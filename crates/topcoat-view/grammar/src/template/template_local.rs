use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Stmt, Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::hir::{AttributeBuilder, LowerAttribute},
    view::hir::{LowerView, ViewBuilder},
};

/// A `let pat = expr;` binding in view-body position. The binding is in scope
/// for all sibling nodes that follow it.
pub struct TemplateLocal {
    pub let_token: Token![let],
    pub body: TokenStream,
    pub semi_token: Token![;],
}

impl TemplateLocal {
    fn parse_body(input: ParseStream) -> syn::Result<TokenStream> {
        let mut tokens = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            // Nested groups are opaque, including any semicolons within them.
            tokens.extend([input.parse::<TokenTree>()?]);
        }
        Ok(tokens)
    }

    fn expansion_tokens(&self) -> TokenStream {
        let original = self.to_token_stream();
        let error = Self::validate(original.clone())
            .err()
            .map(|error| error.to_compile_error());
        quote! { #original #error }
    }

    fn validate(tokens: TokenStream) -> syn::Result<()> {
        // Incomplete Rust is emitted unchanged so the IDE can recover it.
        let Ok(Stmt::Local(local)) = syn::parse2(tokens) else {
            return Ok(());
        };
        let Some(init) = &local.init else {
            return Err(syn::Error::new_spanned(
                &local,
                "`let` binding requires an initializer",
            ));
        };
        if let Some((else_token, _)) = &init.diverge {
            return Err(syn::Error::new_spanned(
                else_token,
                "`let ... else` is not supported in a view",
            ));
        }

        Ok(())
    }
}

impl LowerView for TemplateLocal {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.local_binding(self.expansion_tokens());
    }
}

impl LowerAttribute for TemplateLocal {
    fn lower(&self, builder: &mut AttributeBuilder) {
        builder.local_binding(self.expansion_tokens());
    }
}

impl Parse for TemplateLocal {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            let_token: input.parse()?,
            body: input.call(Self::parse_body)?,
            semi_token: input.parse()?,
        })
    }
}

impl ToTokens for TemplateLocal {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.let_token.to_tokens(tokens);
        self.body.to_tokens(tokens);
        self.semi_token.to_tokens(tokens);
    }
}

impl ParseOption for TemplateLocal {
    fn peek(input: ParseStream) -> bool {
        // `let=` is an attribute named `let`, not the start of a binding.
        input.peek(Token![let]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TemplateLocal {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match syn::parse2::<Stmt>(self.to_token_stream()) {
            Ok(local) => local.pretty_print(printer),
            Err(_) => syn::Expr::Verbatim(self.to_token_stream()).pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    fn parse(source: &str) -> TemplateLocal {
        syn::parse_str(source).unwrap()
    }

    fn binding_strings(source: &str) -> (String, String) {
        let Stmt::Local(local) = syn::parse2(parse(source).to_token_stream()).unwrap() else {
            panic!("expected a local binding");
        };
        (
            local.pat.to_token_stream().to_string(),
            local.init.unwrap().expr.to_token_stream().to_string(),
        )
    }

    #[test]
    fn parses_identifier_binding() {
        let (pat, expr) = binding_strings("let x = 1;");
        assert_eq!(pat, "x");
        assert_eq!(expr, "1");
    }

    #[test]
    fn parses_destructuring_pattern() {
        let (pat, _) = binding_strings("let (a, b) = pair;");
        assert_eq!(pat, "(a , b)");
    }

    #[test]
    fn parses_type_annotation() {
        let (pat, expr) = binding_strings("let x: f64 = 1.0;");
        assert_eq!(pat, "x : f64");
        assert_eq!(expr, "1.0");
    }

    #[test]
    fn parses_initializer_with_low_precedence_operators() {
        // The initializer is a full expression, so `&&`, `||`, and `..` (which a
        // `syn::ExprLet` would stop before) all belong to the bound value.
        assert_eq!(
            binding_strings("let both = true && true;").1,
            "true && true"
        );
        assert_eq!(
            binding_strings("let either = true || false;").1,
            "true || false",
        );
        assert_eq!(binding_strings("let r = 0..10;").1, "0 .. 10");
    }

    #[test]
    fn requires_trailing_semicolon() {
        assert!(syn::parse_str::<TemplateLocal>("let x = 1").is_err());
    }

    #[test]
    fn requires_initializer() {
        let local = parse("let x;");
        assert!(TemplateLocal::validate(local.to_token_stream()).is_err());
        let expanded = local.expansion_tokens().to_string();
        assert!(expanded.starts_with(&local.to_token_stream().to_string()));
        assert!(expanded.contains("compile_error"));
    }

    #[test]
    fn rejects_let_else() {
        let local = parse("let Some(x) = opt else { return; };");
        assert!(TemplateLocal::validate(local.to_token_stream()).is_err());
        let expanded = local.expansion_tokens().to_string();
        assert!(expanded.starts_with(&local.to_token_stream().to_string()));
        assert!(expanded.contains("compile_error"));
    }

    #[test]
    fn forwards_incomplete_bindings() {
        for source in [
            "let x = value.;",
            "let x = call(,);",
            "let x: types:: = value;",
            "let variants:: = value;",
            "let x = { let broken = ; value. };",
        ] {
            let local = parse(source);
            let original: TokenStream = source.parse().unwrap();
            assert_eq!(local.expansion_tokens().to_string(), original.to_string());
        }
    }

    #[test]
    fn stops_after_the_outer_semicolon() {
        let binding = "let x = { let items = [value; 2]; items[0]. };";
        let original: TokenStream = binding.parse().unwrap();
        for source in [
            format!("{binding} <p>(x)</p>"),
            format!("<input {binding} value=(x)>"),
        ] {
            let view = syn::parse_str::<crate::view::View>(&source).unwrap();
            let emit = syn::parse_str::<crate::live::Emit>(&source).unwrap();
            for expanded in [view.to_token_stream(), emit.to_token_stream()] {
                assert!(expanded.to_string().contains(&original.to_string()));
            }
        }
        let attrs =
            syn::parse_str::<crate::attributes::Attributes>(&format!("{binding} value=(x)"))
                .unwrap();
        assert!(
            attrs
                .to_token_stream()
                .to_string()
                .contains(&original.to_string())
        );
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn incomplete_bindings_format_verbatim() {
        use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

        let mut registry = Registry::new();
        registry.register_macro::<crate::view::View>("view");
        registry.register_macro::<crate::live::Emit>("emit");
        registry.register_macro::<crate::attributes::Attributes>("attributes");
        let binding = "let x = value /* keep */ .;";
        for name in ["view", "emit", "attributes"] {
            let source = format!("{name}! {{ {binding} }}");
            let formatted = pretty_print_str(&registry, &source).unwrap();
            assert!(formatted.contains(binding), "{formatted}");
            assert_eq!(pretty_print_str(&registry, &formatted).unwrap(), formatted);
        }
    }

    /// Evaluates a `peek` against `source`, draining the remaining tokens so the
    /// surrounding `parse_str` doesn't error on the unconsumed input.
    fn peeks(peek: fn(ParseStream) -> bool, source: &str) -> bool {
        let parser = move |input: ParseStream| -> syn::Result<bool> {
            let peeked = peek(input);
            input.parse::<proc_macro2::TokenStream>()?;
            Ok(peeked)
        };
        syn::parse::Parser::parse_str(parser, source).unwrap()
    }

    #[test]
    fn let_equals_is_an_attribute_not_a_binding() {
        assert!(peeks(TemplateLocal::peek, "let x = 1;"));
        assert!(!peeks(TemplateLocal::peek, r#"let="x""#));
    }
}
