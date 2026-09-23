use syn::{
    Expr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::{
    template::TemplateForLoop,
    view::{
        Nodes,
        hir::{LowerView, ViewBuilder},
    },
};

/// A `for` loop in a view, with an optional `#[key(expr)]` attribute that
/// gives each iteration its own identity.
///
/// Parsing rejects any other attribute on the loop, and more than one key.
pub struct ForLoop {
    /// The loop itself.
    pub template: TemplateForLoop<Nodes>,
}

impl ForLoop {
    /// Returns the key expression from the loop's `#[key(expr)]` attribute,
    /// or [`None`] for an unkeyed loop.
    ///
    /// # Panics
    ///
    /// Panics if the attributes were changed after parsing and the first one
    /// no longer holds an expression.
    #[must_use]
    pub fn key(&self) -> Option<Expr> {
        self.template
            .attributes
            .first()
            .map(|attribute| attribute.parse_args().expect("validated loop key"))
    }
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let loop_ = Self {
            template: input.parse()?,
        };
        for (index, attribute) in loop_.template.attributes.iter().enumerate() {
            if !attribute.path().is_ident("key") {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "expected `#[key(expr)]` on a view loop",
                ));
            }
            if index > 0 {
                return Err(syn::Error::new_spanned(attribute, "duplicate loop key"));
            }
            attribute.parse_args::<Expr>()?;
        }
        Ok(loop_)
    }
}

impl ParseOption for ForLoop {
    fn peek(input: ParseStream) -> bool {
        TemplateForLoop::<Nodes>::peek(input)
    }
}

impl LowerView for ForLoop {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.for_loop(
            &self.template.pat,
            &self.template.expr,
            self.key(),
            |body| {
                self.template.body.lower(body);
            },
        );
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for ForLoop {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.template.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn extracts_key_expression() {
        let loop_: ForLoop =
            syn::parse_str("#[key(item.id)] for item in items { card() }").unwrap();
        assert_eq!(
            loop_.key().unwrap().to_token_stream().to_string(),
            "item . id"
        );
    }

    #[test]
    fn key_is_optional() {
        let loop_: ForLoop = syn::parse_str("for item in items {}").unwrap();
        assert!(loop_.key().is_none());
    }

    #[test]
    fn rejects_invalid_attributes() {
        for attribute in [
            "#[other(item)]",
            "#[key]",
            "#[key()]",
            "#[key(a, b)]",
            "#[key($(item))]",
            "#[key(item)] #[key(item)]",
        ] {
            let source = format!("{attribute} for item in items {{}}");
            assert!(syn::parse_str::<ForLoop>(&source).is_err(), "{source}");
        }
    }
}
