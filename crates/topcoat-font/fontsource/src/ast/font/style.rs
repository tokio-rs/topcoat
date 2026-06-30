use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    font::List,
    font_face::{StyleKey, StyleValue},
};

/// A `style:` argument for `fontsource_font!`: one style or a bracketed list of
/// styles to cross-product, e.g. `style: [Style::Normal, Style::Italic]`.
pub struct Style {
    pub key: StyleKey,
    pub colon_token: Token![:],
    pub value: List<StyleValue>,
}

impl Parse for Style {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Style {
    fn peek(input: ParseStream) -> bool {
        StyleKey::peek(input)
    }
}
