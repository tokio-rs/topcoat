use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::{
    LitInt, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(weight);
}

/// A `weight:` argument carrying the single weight one face ships, e.g.
/// `weight: 400`.
pub struct Weight {
    pub key: WeightKey,
    pub colon_token: Token![:],
    pub value: WeightValue,
}

impl Parse for Weight {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Weight {
    fn peek(input: ParseStream) -> bool {
        WeightKey::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Weight {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct WeightKey {
    pub weight_kw: kw::weight,
}

impl Parse for WeightKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            weight_kw: input.parse()?,
        })
    }
}

impl ParseOption for WeightKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::weight)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for WeightKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.weight_kw.span().start());
        "weight".pretty_print(printer);
        printer.move_cursor(self.weight_kw.span().end());
    }
}

/// A single weight. Keeps the [`LitInt`] so it is emitted verbatim, with its
/// span, into a [`u16`] position.
pub struct WeightValue(LitInt);

impl WeightValue {
    /// The weight as a number, or `None` when the literal does not fit a
    /// [`u16`]: the compiler reports those on the emitted literal.
    #[must_use]
    pub fn value(&self) -> Option<u16> {
        self.0.base10_parse().ok()
    }

    /// The `compile_error!` for this weight, when the family does not ship it.
    #[must_use]
    pub fn check(&self, family: &runtime::Family) -> Option<TokenStream> {
        let value = self.value()?;
        if family.has_weight(value) {
            return None;
        }
        let available = family
            .weights
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!(
            "`{}` does not ship weight `{value}`; available: {available}",
            family.name
        );
        Some(quote_spanned! {self.0.span()=> ::core::compile_error!(#message); })
    }
}

impl Parse for WeightValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for WeightValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for WeightValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
