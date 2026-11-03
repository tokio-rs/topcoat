use proc_macro2::TokenStream;
use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

use crate::common::lenient::Expr;

/// A `name: value` field, as in a struct literal.
///
/// The name can be any identifier, including a keyword. The colon and the
/// value can be missing, so a field that is still being typed keeps the macro
/// parsing. A missing value is an empty [`Expr`], which expands to a compile
/// error.
#[derive(Clone)]
pub struct FieldValue {
    pub member: Ident,
    pub colon_token: Option<Token![:]>,
    pub expr: Expr,
}

impl FieldValue {
    /// The name as written, without a raw identifier prefix.
    #[must_use]
    pub fn name(&self) -> String {
        self.member.unraw().to_string()
    }

    /// The name as an identifier that is valid in generated code, with
    /// keywords written as raw identifiers.
    #[must_use]
    pub fn ident(&self) -> Ident {
        let name = self.name();
        let is_plain = syn::parse_str::<Ident>(&name).is_ok();
        // These keywords cannot be raw identifiers.
        let cannot_be_raw = matches!(name.as_str(), "_" | "crate" | "self" | "super" | "Self");
        if is_plain || cannot_be_raw {
            self.member.clone()
        } else {
            Ident::new_raw(&name, self.member.span())
        }
    }
}

impl Parse for FieldValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let member = input.call(Ident::parse_any)?;
        // `name::x` is a path, not a field.
        let colon_token = if input.peek(Token![:]) && !input.peek(Token![::]) {
            Some(input.parse()?)
        } else {
            None
        };
        let expr = match colon_token {
            Some(_) => input.parse()?,
            None => Expr {
                tokens: TokenStream::new(),
                span: input.span(),
            },
        };
        Ok(Self {
            member,
            colon_token,
            expr,
        })
    }
}

#[cfg(feature = "pretty")]
impl crate::common::pretty::PrettyPrint for FieldValue {
    fn pretty_print(&self, printer: &mut crate::common::pretty::Printer<'_>) {
        self.member.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        if !self.expr.is_empty() {
            " ".pretty_print(printer);
            self.expr.pretty_print(printer);
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::punctuated::Punctuated;

    use super::*;

    fn parse(input: &str) -> syn::Result<Punctuated<FieldValue, Token![,]>> {
        syn::parse::Parser::parse_str(Punctuated::parse_terminated, input)
    }

    #[test]
    fn the_value_ends_at_the_next_comma() {
        let fields = parse("a: f(x, y), b: 1 + 2").unwrap();
        let names: Vec<_> = fields.iter().map(FieldValue::name).collect();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(fields[0].expr, syn::parse_quote!(f(x, y)));
        assert_eq!(fields[1].expr, syn::parse_quote!(1 + 2));
    }

    #[test]
    fn parses_fields_being_typed() {
        for input in ["na", "name:", "name:, other: 1", "type: 1", "r#type: 1"] {
            assert!(parse(input).is_ok(), "{input}");
        }
    }

    #[test]
    fn a_missing_value_expands_to_an_error() {
        for input in ["name", "name:"] {
            let fields = parse(input).unwrap();
            let expanded = fields[0].expr.to_token_stream().to_string();
            assert!(expanded.contains("compile_error"), "{input}");
        }
    }

    #[test]
    fn a_path_is_not_a_field() {
        assert!(parse("name::x").is_err());
    }

    #[test]
    fn keywords_become_raw_identifiers() {
        for input in ["type: 1", "r#type: 1"] {
            let field = &parse(input).unwrap()[0];
            assert_eq!(field.name(), "type");
            let ident = field.ident();
            assert!(
                syn::parse2::<Ident>(ident.to_token_stream()).is_ok(),
                "{input}"
            );
        }
        assert_eq!(parse("name: 1").unwrap()[0].ident(), "name");
    }
}
