use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::common::{ParseOption, lenient, paths::topcoat_font};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(format);
}

/// A `format(...)` hint on a CSS `@font-face` `src` entry.
///
/// Accepts a CSS keyword string such as `"woff2"` or an expression that
/// evaluates to [`topcoat_font::FontFormat`].
pub struct FontFormatHint {
    pub format_kw: kw::format,
    pub paren_token: Paren,
    pub value: FontFormat,
}

impl Parse for FontFormatHint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            format_kw: input.parse()?,
            paren_token: parenthesized!(content in input),
            value: content.parse()?,
        })
    }
}

impl ParseOption for FontFormatHint {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::format)
    }
}

impl ToTokens for FontFormatHint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontFormatHint {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::common::pretty::Delim;

        printer.move_cursor(self.format_kw.span().start());
        "format".pretty_print(printer);
        self.paren_token.pretty_print(printer, None, |printer| {
            self.value.pretty_print(printer);
        });
    }
}

/// The format inside a [`FontFormatHint`].
///
/// A string literal is a CSS keyword, and an unknown keyword expands to a
/// compile error. Other expressions must evaluate to
/// [`topcoat_font::FontFormat`].
pub struct FontFormat(pub lenient::Expr);

impl Parse for FontFormat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for FontFormat {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Ok(keyword) = syn::parse2::<LitStr>(self.0.tokens.clone()) else {
            self.0.to_tokens(tokens);
            return;
        };
        match format_variant(&keyword.value()) {
            Some(name) => {
                let variant = proc_macro2::Ident::new(name, keyword.span());
                quote! { #topcoat_font::FontFormat::#variant }.to_tokens(tokens);
            }
            None => syn::Error::new_spanned(
                &keyword,
                format!("`{}` is not a valid font format", keyword.value()),
            )
            .to_compile_error()
            .to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontFormat {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}

/// Returns the `FontFormat` variant for a CSS keyword, or `None` if unknown.
fn format_variant(keyword: &str) -> Option<&'static str> {
    Some(match keyword {
        "collection" => "Collection",
        "embedded-opentype" => "EmbeddedOpenType",
        "opentype" => "OpenType",
        "svg" => "Svg",
        "truetype" => "TrueType",
        "woff" => "Woff",
        "woff2" => "Woff2",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(source: &str) -> String {
        syn::parse_str::<FontFormat>(source)
            .unwrap()
            .to_token_stream()
            .to_string()
    }

    #[test]
    fn expands_a_keyword_to_its_variant() {
        assert!(expand(r#""woff2""#).ends_with(":: FontFormat :: Woff2"));
    }

    #[test]
    fn expands_an_unknown_keyword_to_a_compile_error() {
        assert!(expand(r#""wof""#).contains("compile_error"));
    }

    #[test]
    fn keeps_an_incomplete_expression() {
        assert_eq!(expand("format."), "format .");
    }
}
