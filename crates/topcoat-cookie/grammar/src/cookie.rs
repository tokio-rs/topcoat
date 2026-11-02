mod attribute;
mod header_name;
mod value;

pub use attribute::*;
pub use header_name::*;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::paths::topcoat_cookie;
pub use value::*;

/// Arguments to `cookie!`, written like a `Set-Cookie` header: the cookie's
/// `name = value`, then attributes separated by `;`.
///
/// ```text
/// cookie!(theme = "dark")
/// cookie!(__Host-session = (token); Path = "/"; Secure)
/// ```
pub struct Cookie {
    pub name: CookieName,
    pub eq_token: Token![=],
    pub value: CookieValue,
    pub semi_token: Option<Token![;]>,
    pub attributes: Punctuated<CookieAttribute, Token![;]>,
}

/// Builds the cookie and applies its attributes in order. Errors are reported next
/// to a valid `Cookie` expression.
impl ToTokens for Cookie {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cookie = Ident::new("__topcoat_cookie", Span::mixed_site());
        let name = self.name.expr();
        let value = self.value.expr().unwrap_or_else(|error| {
            let error = error.to_compile_error();
            quote! {{ #error; "" }}
        });
        let attributes = self
            .attributes
            .iter()
            .map(|attribute| attribute.apply(&cookie));
        tokens.extend(quote! {{
            #[allow(unused_mut)]
            let mut #cookie = #topcoat_cookie::Cookie::new(#name, #value);
            #(#attributes)*
            #cookie
        }});
    }
}

impl Parse for Cookie {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let eq_token = input.parse()?;
        let value = input.parse()?;
        let semi_token: Option<Token![;]> = input.parse()?;
        let attributes = match semi_token {
            Some(_) => Punctuated::parse_terminated(input)?,
            None => Punctuated::new(),
        };
        Ok(Self {
            name,
            eq_token,
            value,
            semi_token,
            attributes,
        })
    }
}

/// Prints the cookie on one line if it fits. Otherwise, puts the name and value and
/// each attribute on their own lines, followed by a `;`.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Cookie {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::TextMode;

        self.name.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);

        let Some(semi_token) = &self.semi_token else {
            return;
        };
        if self.attributes.is_empty() {
            printer.move_cursor(semi_token.span.end());
            return;
        }

        semi_token.pretty_print(printer);
        for (index, pair) in self.attributes.pairs().enumerate() {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
            pair.value().pretty_print(printer);
            if pair.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            match pair.punct() {
                Some(punct) if index < self.attributes.len() - 1 => punct.pretty_print(printer),
                _ => {
                    printer.scan_text(";".into(), TextMode::Break);
                    printer.advance_cursor(";");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<Cookie> {
        syn::parse_str(input)
    }

    fn expand(input: &str) -> String {
        parse(input).unwrap().to_token_stream().to_string()
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn parses_the_name_value_and_attributes() {
        let cookie = parse(r#"__Host-session = (token); Path = "/"; Secure;"#).unwrap();
        assert!(
            matches!(&cookie.name, CookieName::Header(name) if name.value() == "__Host-session")
        );
        assert!(matches!(cookie.value, CookieValue::Expr(_)));
        let names: Vec<_> = cookie
            .attributes
            .iter()
            .map(|attribute| attribute.name.value())
            .collect();
        assert_eq!(names, ["Path", "Secure"]);
    }

    #[test]
    fn parses_literal_and_expression_names() {
        assert!(matches!(
            parse(r#""my.cookie" = "x""#).unwrap().name,
            CookieName::Lit(_)
        ));
        assert!(matches!(
            parse(r#"(name()) = "x""#).unwrap().name,
            CookieName::Expr(_)
        ));
    }

    #[test]
    fn parses_attributes_being_typed() {
        for input in [
            r#"a = "b"; Pa"#,
            r#"a = "b"; Path ="#,
            r#"a = "b"; Path =; Secure"#,
        ] {
            assert!(parse(input).is_ok(), "{input}");
        }
    }

    #[test]
    fn rejects_attributes_without_a_separator() {
        assert!(parse(r#"a = "b" Secure"#).is_err());
        assert!(parse(r#"a = "b"; Secure HttpOnly"#).is_err());
    }

    #[test]
    fn writes_a_header_name_as_a_string() {
        assert!(expand(r#"__Host - session = "x""#).contains(r#""__Host-session""#));
    }

    #[test]
    fn a_flag_on_its_own_is_on() {
        assert!(expand(r#"a = "b"; Secure"#).contains(&tokens("set_secure (true)")));
    }

    #[test]
    fn an_integer_max_age_counts_seconds() {
        assert!(expand(r#"a = "b"; Max-Age = 90"#).contains(&tokens("Duration :: seconds (90)")));
    }

    #[test]
    fn a_same_site_name_is_a_variant() {
        assert!(expand(r#"a = "b"; SameSite = Lax"#).contains(&tokens("SameSite :: Lax")));
    }

    #[test]
    fn passes_expressions_on_without_their_parentheses() {
        // Parentheses around an argument trigger the `unused_parens` lint in the
        // caller's code.
        let expanded = expand("a = (token); Path = (path)");
        assert!(
            expanded.contains(&tokens("Cookie :: new (\"a\" , token)")),
            "{expanded}"
        );
        assert!(expanded.contains(&tokens("set_path (path)")), "{expanded}");
    }

    #[test]
    fn reports_invalid_attributes_and_still_builds_the_cookie() {
        for input in [
            r#"a = "b"; Unknown = "x""#,
            r#"a = "b"; Path"#,
            r#"a = "b"; Path ="#,
            r#"a = "b"; Path = root"#,
            "a = b",
        ] {
            let expanded = expand(input);
            assert!(expanded.contains("compile_error"), "{input}");
            assert!(expanded.contains(&tokens("Cookie :: new")), "{input}");
        }
    }

    #[test]
    fn applies_valid_attributes_next_to_an_invalid_one() {
        let expanded = expand(r#"a = "b"; Unknown; Path = "/""#);
        assert!(expanded.contains("compile_error"), "{expanded}");
        assert!(expanded.contains("set_path"), "{expanded}");
    }
}
