use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(family);
}

pub struct FontFamily {
    pub key: FontFamilyKey,
    pub colon_token: Token![:],
    pub value: FontFamilyValue,
}

impl Parse for FontFamily {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontFamily {
    fn peek(input: ParseStream) -> bool {
        FontFamilyKey::peek(input)
    }
}

pub struct FontFamilyKey {
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub family_kw: kw::family,
}

impl Parse for FontFamilyKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            font_kw: input.parse()?,
            dash_token: input.parse()?,
            family_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontFamilyKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::font) && input.peek2(Token![-]) && input.peek3(kw::family)
    }
}

pub struct FontFamilyValue(pub Expr);

impl Parse for FontFamilyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}
