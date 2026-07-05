use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(subset);
}

/// A `subset:` argument carrying the single subset one face ships, e.g.
/// `subset: Latin`.
pub struct Subset {
    pub key: SubsetKey,
    pub colon_token: Token![:],
    pub value: SubsetValue,
}

impl Parse for Subset {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Subset {
    fn peek(input: ParseStream) -> bool {
        SubsetKey::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Subset {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct SubsetKey {
    pub subset_kw: kw::subset,
}

impl Parse for SubsetKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            subset_kw: input.parse()?,
        })
    }
}

impl ParseOption for SubsetKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::subset)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for SubsetKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.subset_kw.span().start());
        "subset".pretty_print(printer);
        printer.move_cursor(self.subset_kw.span().end());
    }
}

/// A single subset, written as a bare variant name (`Latin`).
///
/// Emits the [`Subset`](runtime::Subset) variant's path, keeping the written
/// ident's span so the compiler reports unknown variants on it and editors
/// autocomplete them.
pub struct SubsetValue(Ident);

impl SubsetValue {
    /// The written ident, e.g. `LatinExt`.
    #[must_use]
    pub fn ident(&self) -> &Ident {
        &self.0
    }

    /// The subset the written variant names, or `None` when it is not a
    /// [`Subset`](runtime::Subset) variant: the compiler reports those on the
    /// emitted variant.
    #[must_use]
    pub fn subset(&self) -> Option<runtime::Subset> {
        runtime::Subset::from_variant(&self.0.to_string())
    }

    /// The `compile_error!` for this subset, when the family does not ship it.
    #[must_use]
    pub fn check(&self, family: &runtime::Family) -> Option<TokenStream> {
        let subset = self.subset()?;
        if family.has_subset(subset) {
            return None;
        }
        let available = family
            .subsets
            .iter()
            .map(|subset| format!("{subset:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!(
            "`{}` does not ship the `{}` subset; available: {available}",
            family.name, self.0,
        );
        Some(quote_spanned! {self.0.span()=> ::core::compile_error!(#message); })
    }
}

impl Parse for SubsetValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for SubsetValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.0;
        quote::quote! { ::topcoat::font::fontsource::Subset::#ident }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for SubsetValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
