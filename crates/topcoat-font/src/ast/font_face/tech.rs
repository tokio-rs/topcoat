use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, LitStr, parenthesized,
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
pub enum FontTech {
    Keyword(LitStr),
    Expr(Box<Expr>),
}

impl Parse for FontTech {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let keyword: LitStr = input.parse()?;
            if runtime::FontTech::from_keyword(&keyword.value()).is_none() {
                return Err(syn::Error::new_spanned(
                    &keyword,
                    format!("`{}` is not a valid font technology", keyword.value()),
                ));
            }
            Ok(Self::Keyword(keyword))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for FontTech {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Keyword(keyword) => quote! {
                ::topcoat::font::FontTech::from_keyword(#keyword).unwrap()
            },
            Self::Expr(inner) => quote! {
                ::topcoat::font::FontTech::from(#inner)
            },
        }
        .to_tokens(tokens);
    }
}
