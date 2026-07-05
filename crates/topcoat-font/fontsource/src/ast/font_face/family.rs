use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident,
    parse::{Parse, ParseStream},
};

use crate::runtime::Family;

/// A Fontsource font family, written as the name of its
/// [`families`](crate::runtime::families) constant (e.g. `ROBOTO`).
///
/// Emits the constant's path, keeping the written ident's span so the compiler
/// reports unknown names on it and editors autocomplete the catalog.
pub struct FamilyName(Ident);

impl FamilyName {
    /// The written ident, e.g. `ROBOTO`.
    #[must_use]
    pub fn ident(&self) -> &Ident {
        &self.0
    }

    /// The catalog family the written name refers to, or `None` when there is
    /// no [`families`](crate::runtime::families) constant of that name: the
    /// emitted path then fails to resolve and the compiler reports it.
    #[must_use]
    pub fn family(&self) -> Option<&'static Family> {
        Family::by_ident(&self.0.to_string())
    }
}

impl Parse for FamilyName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for FamilyName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.0;
        quote! { ::topcoat::font::fontsource::families::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FamilyName {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
