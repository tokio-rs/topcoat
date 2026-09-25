use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::paths::topcoat_font_fontsource;
use topcoat_font::fontsource::Family;

/// A Fontsource font family, written as the name of its
/// [`families`](topcoat_font::fontsource::families) constant (e.g. `ROBOTO`).
///
/// Preserves the identifier's span for compiler errors and editor completion.
pub struct FamilyName(Ident);

impl FamilyName {
    /// The written ident, e.g. `ROBOTO`.
    #[must_use]
    pub fn ident(&self) -> &Ident {
        &self.0
    }

    /// Returns the named family, or `None` if it is absent from the catalog.
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
        quote! { #topcoat_font_fontsource::families::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FamilyName {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
