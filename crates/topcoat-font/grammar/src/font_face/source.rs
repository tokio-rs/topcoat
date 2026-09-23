use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};
use topcoat_core_grammar::{ParseOption, QuoteOption, paths::topcoat_font};

use crate::font_face::{FontFormatHint, FontTechHint};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(src);
    custom_keyword!(url);
    custom_keyword!(local);
}

/// A `src: ...` descriptor in a `font_face!` body.
pub struct FontSources {
    /// The `src` name.
    pub key: FontSourcesKey,
    /// The `:` between the name and the value.
    pub colon_token: Token![:],
    /// The descriptor's value.
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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontSources {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

/// The `src` descriptor name.
pub struct FontSourcesKey {
    /// The `src` keyword.
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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontSourcesKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.src_kw.span().start());
        "src".pretty_print(printer);
        printer.move_cursor(self.src_kw.span().end());
    }
}

/// The value of a `src` descriptor: CSS syntax or a Rust expression.
pub enum FontSourcesValue {
    /// A Rust expression that evaluates to `FontSources`.
    Expr(Box<Expr>),
    /// A comma-separated list of `url(...)` and `local(...)` entries.
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
                #topcoat_font::FontSources::new(::std::vec![#inner])
            }
            .to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontSourcesValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::BreakMode;

        match self {
            Self::Expr(inner) => inner.pretty_print(printer),
            Self::Css(sources) => {
                printer.scan_begin(BreakMode::Inconsistent);
                sources.pretty_print(printer);
                printer.scan_end();
            }
        }
    }
}

/// A single `url(...)` or `local(...)` entry of a `src` descriptor.
pub enum FontSource {
    /// A `url(...)` entry with optional `format(...)` and `tech(...)` hints.
    Url {
        /// The `url` keyword.
        url_kw: kw::url,
        /// The parentheses around the URL.
        paren_token: Paren,
        /// The URL: a string, a `String`, or an `Asset`.
        expr: Expr,

        /// The `tech(...)` hint, if written.
        tech: Option<Box<FontTechHint>>,
        /// The `format(...)` hint, if written.
        format: Option<Box<FontFormatHint>>,
    },
    /// A `local(...)` entry naming an installed font.
    Local {
        /// The `local` keyword.
        local_kw: kw::local,
        /// The parentheses around the name.
        paren_token: Paren,
        /// The family name of the installed font.
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
                quote! { #topcoat_font::FontSource::url(#expr, #format, #tech) }
            }
            Self::Local { expr, .. } => {
                quote! { #topcoat_font::FontSource::local(#expr) }
            }
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontSource {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::pretty::Delim;

        match self {
            Self::Url {
                url_kw,
                paren_token,
                expr,
                tech,
                format,
            } => {
                printer.move_cursor(url_kw.span().start());
                "url".pretty_print(printer);
                paren_token.pretty_print(printer, None, |printer| {
                    expr.pretty_print(printer);
                });
                // `format(...)` precedes `tech(...)` in the CSS grammar; emit
                // them in that canonical order regardless of source order.
                if let Some(format) = format {
                    " ".pretty_print(printer);
                    format.pretty_print(printer);
                }
                if let Some(tech) = tech {
                    " ".pretty_print(printer);
                    tech.pretty_print(printer);
                }
            }
            Self::Local {
                local_kw,
                paren_token,
                expr,
            } => {
                printer.move_cursor(local_kw.span().start());
                "local".pretty_print(printer);
                paren_token.pretty_print(printer, None, |printer| {
                    expr.pretty_print(printer);
                });
            }
        }
    }
}
