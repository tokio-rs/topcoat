use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_font_fontsource;

use topcoat_font::fontsource as runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(style);
}

/// A `style:` argument carrying the single style one face ships, e.g.
/// `style: Italic`.
pub struct Style {
    pub key: StyleKey,
    pub colon_token: Token![:],
    pub value: StyleValue,
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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Style {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct StyleKey {
    pub style_kw: kw::style,
}

impl Parse for StyleKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            style_kw: input.parse()?,
        })
    }
}

impl ParseOption for StyleKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::style)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for StyleKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.style_kw.span().start());
        "style".pretty_print(printer);
        printer.move_cursor(self.style_kw.span().end());
    }
}

/// A single style, written as a bare variant name (`Normal` or `Italic`).
///
/// Emits the [`Style`](runtime::Style) variant's path, keeping the written
/// ident's span so the compiler reports unknown variants on it and editors
/// autocomplete them.
pub struct StyleValue(Ident);

impl StyleValue {
    /// The written ident, e.g. `Normal`.
    #[must_use]
    pub fn ident(&self) -> &Ident {
        &self.0
    }

    /// The style the written variant names, or `None` when it is not a
    /// [`Style`](runtime::Style) variant: the compiler reports those on the
    /// emitted variant.
    #[must_use]
    pub fn style(&self) -> Option<runtime::Style> {
        match self.ident().to_string().as_str() {
            "Normal" => Some(runtime::Style::Normal),
            "Italic" => Some(runtime::Style::Italic),
            _ => None,
        }
    }

    /// The `compile_error!` for this style, when the family does not ship it.
    #[must_use]
    pub fn check(&self, family: &runtime::Family) -> Option<TokenStream> {
        let style = self.style()?;
        if family.has_style(style) {
            return None;
        }
        let message = format!(
            "`{}` does not ship the `{}` style",
            family.name,
            self.ident()
        );
        Some(quote_spanned! {self.0.span()=> ::core::compile_error!(#message); })
    }
}

impl Parse for StyleValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for StyleValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.0;
        quote::quote! { #topcoat_font_fontsource::Style::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for StyleValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
