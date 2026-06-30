use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};

use topcoat_core::ast::{ParseOption, QuoteOption};

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

impl ToTokens for FontSources {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
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

impl ToTokens for FontSourcesValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(inner) => inner.to_tokens(tokens),
            Self::Css(inner) => quote! {
                ::topcoat::font::FontSources::new(::std::vec![#inner])
            }
            .to_tokens(tokens),
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

impl ToTokens for FontSource {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Url {
                expr, tech, format, ..
            } => {
                let tech = QuoteOption::from(tech);
                let format = QuoteOption::from(format);
                quote! { ::topcoat::font::FontSource::url(#expr, #format, #tech) }
            }
            Self::Local { expr, .. } => {
                quote! { ::topcoat::font::FontSource::local(#expr) }
            }
        }
        .to_tokens(tokens);
    }
}
