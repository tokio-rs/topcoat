use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

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
impl topcoat_pretty::PrettyPrint for Host {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
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
impl topcoat_pretty::PrettyPrint for HostKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.host_kw.span().start());
        "host".pretty_print(printer);
        printer.move_cursor(self.host_kw.span().end());
    }
}

/// The sole host value, the `asset` keyword.
pub struct HostValue {
    pub path: syn::Path,
}

impl HostValue {
    /// The host the parsed path refers to.
    ///
    /// # Panics
    ///
    /// Panics if the parsed path has no segments, which a successfully parsed
    /// [`syn::Path`] never does.
    #[must_use]
    pub fn host(&self) -> runtime::Host {
        match self
            .path
            .segments
            .last()
            .expect("parsed path has at least one segment")
            .ident
            .to_string()
            .as_str()
        {
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
        Ok(Self {
            path: input.parse()?,
        })
    }
}

impl ToTokens for HostValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &self.path;
        quote! {
            ::topcoat::font::fontsource::Host::#path
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for HostValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.path.pretty_print(printer);
    }
}
