use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(display);
    custom_keyword!(auto);
    custom_keyword!(block);
    custom_keyword!(swap);
    custom_keyword!(fallback);
    custom_keyword!(optional);
}

pub struct FontDisplay {
    pub key: FontDisplayKey,
    pub colon_token: Token![:],
    pub value: FontDisplayValue,
}

impl Parse for FontDisplay {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontDisplay {
    fn peek(input: ParseStream) -> bool {
        FontDisplayKey::peek(input)
    }
}

impl ToTokens for FontDisplay {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

pub struct FontDisplayKey {
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub display_kw: kw::display,
}

impl Parse for FontDisplayKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            font_kw: input.parse()?,
            dash_token: input.parse()?,
            display_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontDisplayKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::font) && input.peek2(Token![-]) && input.peek3(kw::display)
    }
}

pub enum FontDisplayValue {
    Expr(Box<Expr>),
    Css(FontDisplayKind),
}

impl Parse for FontDisplayValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if FontDisplayKind::peek(input) {
            Ok(Self::Css(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for FontDisplayValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Css(kind) => kind.to_tokens(tokens),
            Self::Expr(inner) => inner.to_tokens(tokens),
        }
    }
}

/// The display strategy of a font face: `auto`, `block`, `swap`, `fallback`, or
/// `optional`.
pub enum FontDisplayKind {
    Auto(kw::auto),
    Block(kw::block),
    Swap(kw::swap),
    Fallback(kw::fallback),
    Optional(kw::optional),
}

impl ParseOption for FontDisplayKind {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::auto)
            || input.peek(kw::block)
            || input.peek(kw::swap)
            || input.peek(kw::fallback)
            || input.peek(kw::optional)
    }
}

impl Parse for FontDisplayKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::auto) {
            Ok(Self::Auto(input.parse()?))
        } else if lookahead.peek(kw::block) {
            Ok(Self::Block(input.parse()?))
        } else if lookahead.peek(kw::swap) {
            Ok(Self::Swap(input.parse()?))
        } else if lookahead.peek(kw::fallback) {
            Ok(Self::Fallback(input.parse()?))
        } else if lookahead.peek(kw::optional) {
            Ok(Self::Optional(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for FontDisplayKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Auto(_) => quote! { ::topcoat::font::FontDisplay::Auto },
            Self::Block(_) => quote! { ::topcoat::font::FontDisplay::Block },
            Self::Swap(_) => quote! { ::topcoat::font::FontDisplay::Swap },
            Self::Fallback(_) => quote! { ::topcoat::font::FontDisplay::Fallback },
            Self::Optional(_) => quote! { ::topcoat::font::FontDisplay::Optional },
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> FontDisplay {
        syn::parse_str(source).unwrap()
    }

    fn css(value: &FontDisplayValue) -> &FontDisplayKind {
        match value {
            FontDisplayValue::Css(kind) => kind,
            FontDisplayValue::Expr(_) => panic!("expected a CSS font-display value"),
        }
    }

    #[test]
    fn parses_every_keyword() {
        assert!(matches!(
            css(&parse("font-display: auto").value),
            FontDisplayKind::Auto(_)
        ));
        assert!(matches!(
            css(&parse("font-display: block").value),
            FontDisplayKind::Block(_)
        ));
        assert!(matches!(
            css(&parse("font-display: swap").value),
            FontDisplayKind::Swap(_)
        ));
        assert!(matches!(
            css(&parse("font-display: fallback").value),
            FontDisplayKind::Fallback(_)
        ));
        assert!(matches!(
            css(&parse("font-display: optional").value),
            FontDisplayKind::Optional(_)
        ));
    }

    #[test]
    fn falls_back_to_an_expression() {
        let display = parse("font-display: (my_display)");
        assert!(matches!(display.value, FontDisplayValue::Expr(_)));
    }
}
