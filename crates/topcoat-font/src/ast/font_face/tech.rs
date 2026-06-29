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

    custom_keyword!(tech);
}

/// A `tech(...)` hint on a CSS `@font-face` `src` entry.
///
/// The technology may be a string literal naming a CSS technology keyword (such
/// as `"color-colrv1"`) or a parenthesized expression resolving to a
/// [`FontTech`] at run time.
pub struct FontTechHint {
    pub tech_kw: kw::tech,
    pub paren_token: Paren,
    pub value: FontTech,
}

impl Parse for FontTechHint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            tech_kw: input.parse()?,
            paren_token: parenthesized!(content in input),
            value: content.parse()?,
        })
    }
}

impl ParseOption for FontTechHint {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::tech)
    }
}

impl ToTokens for FontTechHint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

/// The technology inside a [`FontTechHint`].
///
/// Wraps an expression that resolves to a [`runtime::FontTech`] at run time.
/// When the expression is a string literal, it is validated at compile time
/// against the known CSS technology keywords.
pub struct FontTech(pub Expr);

impl Parse for FontTech {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        if let Expr::Lit(lit) = &expr
            && let Lit::Str(keyword) = &lit.lit
            && runtime::FontTech::from_keyword(&keyword.value()).is_none()
        {
            return Err(syn::Error::new_spanned(
                keyword,
                format!("`{}` is not a valid font technology", keyword.value()),
            ));
        }
        Ok(Self(expr))
    }
}

impl ToTokens for FontTech {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Expr::Lit(lit) = &self.0
            && let Lit::Str(keyword) = &lit.lit
        {
            quote! {
                ::topcoat::font::FontTech::from_keyword(#keyword).unwrap()
            }
            .to_tokens(tokens);
            return;
        }
        let inner = &self.0;
        quote! {
            ::topcoat::font::FontTech::from(#inner)
        }
        .to_tokens(tokens);
    }
}
