use syn::{
    Expr, Local, Pat, Stmt, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::{AttributeWriter, WriteAttribute},
    view::{ViewWriter, WriteView},
};

/// A `let pat = expr;` binding in view-body position. The binding is in scope
/// for all sibling nodes that follow it.
pub struct TemplateLocal {
    pub local: Local,
}

impl TemplateLocal {
    /// The binding's pattern and initializer expression. [`Parse`] guarantees an
    /// initializer is present, so this never panics.
    fn binding(&self) -> (&Pat, &Expr) {
        let init = self
            .local
            .init
            .as_ref()
            .expect("a `let` binding always has an initializer");
        (&self.local.pat, &init.expr)
    }
}

impl WriteView for TemplateLocal {
    fn write(&self, writer: &mut ViewWriter) {
        let (pat, expr) = self.binding();
        writer.local_binding(pat, expr);
    }
}

impl WriteAttribute for TemplateLocal {
    fn write(&self, writer: &mut AttributeWriter) {
        let (pat, expr) = self.binding();
        writer.local_binding(pat, expr);
    }
}

impl Parse for TemplateLocal {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse a statement-position `let`, whose initializer is a full
        // expression. `syn::ExprLet` is the `if let`/`while let` condition form
        // instead, and stops the initializer before `&&`, `||`, `..`, etc. (they
        // would belong to the enclosing let-chain) and rejects type annotations.
        let local = match input.parse()? {
            Stmt::Local(local) => local,
            other => return Err(syn::Error::new_spanned(other, "expected a `let` binding")),
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

        Ok(Self { local })
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
        self.local.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    fn parse(source: &str) -> TemplateLocal {
        syn::parse_str(source).unwrap()
    }

    fn binding_strings(source: &str) -> (String, String) {
        let local = parse(source);
        let (pat, expr) = local.binding();
        (
            pat.to_token_stream().to_string(),
            expr.to_token_stream().to_string(),
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
        assert!(syn::parse_str::<TemplateLocal>("let x;").is_err());
    }

    #[test]
    fn rejects_let_else() {
        assert!(syn::parse_str::<TemplateLocal>("let Some(x) = opt else { return; };").is_err());
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
