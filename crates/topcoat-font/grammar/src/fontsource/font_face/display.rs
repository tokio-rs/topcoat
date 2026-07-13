use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_font;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(display);
}

/// A `display:` argument carrying the face's `font-display` strategy, written as
/// a [`FontDisplay`] variant name, e.g. `display: Swap`. Defaults to `Swap` when
/// omitted.
///
/// [`FontDisplay`]: ../../../font/enum.FontDisplay.html
pub struct Display {
    pub key: DisplayKey,
    pub colon_token: Token![:],
    pub value: DisplayValue,
}

impl Parse for Display {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Display {
    fn peek(input: ParseStream) -> bool {
        DisplayKey::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Display {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct DisplayKey {
    pub display_kw: kw::display,
}

impl Parse for DisplayKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            display_kw: input.parse()?,
        })
    }
}

impl ParseOption for DisplayKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::display)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for DisplayKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.display_kw.span().start());
        "display".pretty_print(printer);
        printer.move_cursor(self.display_kw.span().end());
    }
}

/// A single display strategy, written as a bare variant name (`Swap`).
///
/// Emits the [`FontDisplay`] variant's path, keeping the written ident's span
/// so the compiler reports unknown strategies on it and editors autocomplete
/// them.
///
/// [`FontDisplay`]: ../../../font/enum.FontDisplay.html
pub struct DisplayValue(Ident);

impl DisplayValue {
    /// The written ident, e.g. `Swap`.
    #[must_use]
    pub fn ident(&self) -> &Ident {
        &self.0
    }
}

impl Parse for DisplayValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for DisplayValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.0;
        quote::quote! { #topcoat_font::FontDisplay::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for DisplayValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
