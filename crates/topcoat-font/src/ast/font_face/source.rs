use syn::{
    Expr, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};

use topcoat_core::ast::ParseOption;

use crate::ast::font_face::{FontFormatHint, FontTechHint};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(src);
    custom_keyword!(url);
    custom_keyword!(local);
}

pub struct FontSources {
    pub key: FontSourcesKey,
    pub colon_token: Token![:],
    pub value: FontSourcesValue,
}

impl Parse for FontSources {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontSources {
    fn peek(input: ParseStream) -> bool {
        FontSourcesKey::peek(input)
    }
}

pub struct FontSourcesKey {
    pub src_kw: kw::src,
}

impl Parse for FontSourcesKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            src_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontSourcesKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::src)
    }
}

pub enum FontSourcesValue {
    Expr(Box<Expr>),
    Css(Punctuated<FontSource, Token![,]>),
}

impl Parse for FontSourcesValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::url) || input.peek(kw::local) {
            Ok(Self::Css(Punctuated::parse_separated_nonempty(input)?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

pub enum FontSource {
    Url {
        url_kw: kw::url,
        paren_token: Paren,
        expr: Expr,

        tech: Option<Box<FontTechHint>>,
        format: Option<Box<FontFormatHint>>,
    },
    Local {
        local_kw: kw::local,
        paren_token: Paren,
        expr: Expr,
    },
}

impl Parse for FontSource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::url) {
            let content;
            let url_kw = input.parse()?;
            let paren_token = parenthesized!(content in input);
            let expr = content.parse()?;

            // `format(...)` and `tech(...)` are each optional and accepted in
            // either order.
            let mut format = None;
            let mut tech = None;
            loop {
                if FontFormatHint::peek(input) {
                    if format.is_some() {
                        return Err(input.error("duplicate `format(...)` hint"));
                    }
                    format = Some(Box::new(input.parse()?));
                } else if FontTechHint::peek(input) {
                    if tech.is_some() {
                        return Err(input.error("duplicate `tech(...)` hint"));
                    }
                    tech = Some(Box::new(input.parse()?));
                } else {
                    break;
                }
            }

            Ok(Self::Url {
                url_kw,
                paren_token,
                expr,
                tech,
                format,
            })
        } else if lookahead.peek(kw::local) {
            let content;
            Ok(Self::Local {
                local_kw: input.parse()?,
                paren_token: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

