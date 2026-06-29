use syn::{
    Expr, LitStr, parenthesized,
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

/// The format inside a [`FontFormatHint`].
pub enum FontFormat {
    Keyword(LitStr),
    Expr(Box<Expr>),
}

impl Parse for FontFormat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let keyword: LitStr = input.parse()?;
            if runtime::FontFormat::from_keyword(&keyword.value()).is_none() {
                return Err(syn::Error::new_spanned(
                    &keyword,
                    format!("`{}` is not a valid font format", keyword.value()),
                ));
            }
            Ok(Self::Keyword(keyword))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}
