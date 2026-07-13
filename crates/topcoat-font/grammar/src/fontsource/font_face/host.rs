use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_font_fontsource;

use topcoat_font::fontsource as runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(host);
}

pub struct Host {
    pub key: HostKey,
    pub colon_token: Token![:],
    pub value: HostValue,
}

impl Host {
    #[must_use]
    pub fn host(&self) -> runtime::Host {
        self.value.host()
    }
}

impl Parse for Host {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Host {
    fn peek(input: ParseStream) -> bool {
        HostKey::peek(input)
    }
}

impl ToTokens for Host {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Host {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct HostKey {
    pub host_kw: kw::host,
}

impl Parse for HostKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            host_kw: input.parse()?,
        })
    }
}

impl ParseOption for HostKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::host)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HostKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.host_kw.span().start());
        "host".pretty_print(printer);
        printer.move_cursor(self.host_kw.span().end());
    }
}

/// A single host, written as a bare variant name (`Asset`).
///
/// Emits the [`Host`](runtime::Host) variant's path, keeping the written
/// ident's span so the compiler reports unknown variants on it and editors
/// autocomplete them.
pub struct HostValue(syn::Ident);

impl HostValue {
    /// The written ident, e.g. `Asset`.
    #[must_use]
    pub fn ident(&self) -> &syn::Ident {
        &self.0
    }

    /// The host the written variant names. Unknown names fall back to
    /// [`JsDelivr`](runtime::Host::JsDelivr); the compiler reports them on the
    /// emitted variant.
    #[must_use]
    pub fn host(&self) -> runtime::Host {
        match self.0.to_string().as_str() {
            #[cfg(feature = "asset")]
            "Asset" => runtime::Host::Asset,
            "JsDelivr" => runtime::Host::JsDelivr,
            // For autocomplete to work, be lenient.
            #[allow(clippy::match_same_arms)]
            _ => runtime::Host::JsDelivr,
        }
    }
}

impl Parse for HostValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for HostValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.0;
        quote::quote! { #topcoat_font_fontsource::Host::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HostValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
