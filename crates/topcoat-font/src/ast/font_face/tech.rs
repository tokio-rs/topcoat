use syn::{
    Expr, LitStr, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core::ast::ParseOption;

use crate::runtime::{self};

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
