use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Lit, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

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

/// The format inside a [`FontFormatHint`].
///
/// Wraps an expression that resolves to a [`runtime::FontFormat`] at run time.
/// When the expression is a string literal, it is validated at compile time
/// against the known CSS format keywords.
pub struct FontFormat(pub Expr);

impl Parse for FontFormat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        if let Expr::Lit(lit) = &expr
            && let Lit::Str(keyword) = &lit.lit
            && runtime::FontFormat::from_keyword(&keyword.value()).is_none()
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
            let variant = format_variant(&keyword.value(), keyword.span());
            quote! { ::topcoat::font::FontFormat::#variant }.to_tokens(tokens);
            return;
        }
        let inner = &self.0;
        inner.to_tokens(tokens);
    }
}

fn format_variant(keyword: &str, span: proc_macro2::Span) -> proc_macro2::Ident {
    let name = match keyword {
        "collection" => "Collection",
        "embedded-opentype" => "EmbeddedOpenType",
        "opentype" => "OpenType",
        "svg" => "Svg",
        "truetype" => "TrueType",
        "woff" => "Woff",
        "woff2" => "Woff2",
        _ => unreachable!("validated at parse time"),
    };
    proc_macro2::Ident::new(name, span)
}
