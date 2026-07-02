use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Lit, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(format);
}

/// A `format(...)` hint on a CSS `@font-face` `src` entry.
///
/// The format may be a string literal naming a CSS format keyword (such as
/// `"woff2"`) or a parenthesized expression resolving to a [`FontFormat`] at
/// run time.
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
impl topcoat_pretty::PrettyPrint for FontFormatHint {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_pretty::Delim;

        printer.move_cursor(self.format_kw.span().start());
        "format".pretty_print(printer);
        self.paren_token.pretty_print(printer, None, |printer| {
            self.value.pretty_print(printer);
        });
    }
}

/// The format inside a [`FontFormatHint`].
///
/// Wraps an expression that resolves to a [`crate::runtime::FontFormat`] at run
/// time. When the expression is a string literal, it is validated at compile
/// time against the known CSS format keywords.
pub struct FontFormat(pub Expr);

impl Parse for FontFormat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        if let Expr::Lit(lit) = &expr
            && let Lit::Str(keyword) = &lit.lit
            && format_variant(&keyword.value()).is_none()
        {
            return Err(syn::Error::new_spanned(
                keyword,
                format!("`{}` is not a valid font format", keyword.value()),
            ));
        }
        Ok(Self(expr))
    }
}

impl ToTokens for FontFormat {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Expr::Lit(lit) = &self.0
            && let Lit::Str(keyword) = &lit.lit
        {
            let name = format_variant(&keyword.value()).expect("validated at parse time");
            let variant = proc_macro2::Ident::new(name, keyword.span());
            quote! { ::topcoat::font::FontFormat::#variant }.to_tokens(tokens);
            return;
        }
        let inner = &self.0;
        inner.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontFormat {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}

/// The `FontFormat` variant identifier for a CSS `format(...)` keyword, or
/// `None` if the keyword names no known format.
///
/// This single mapping backs both parse-time validation and codegen, so the two
/// can never disagree.
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
