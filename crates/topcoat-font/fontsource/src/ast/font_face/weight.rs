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

/// A single weight. Keeps the [`LitInt`] so its span drives catalog-validation
/// errors.
pub struct WeightValue(LitInt);

impl WeightValue {
    /// The weight as a number.
    ///
    /// # Errors
    ///
    /// Returns an error if the literal does not parse as a [`u16`].
    pub fn value(&self) -> syn::Result<u16> {
        self.0.base10_parse()
    }

    /// Validates the weight against the family's catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the literal does not parse as a [`u16`], or if the
    /// family does not ship the weight.
    pub fn resolve(&self, family: &runtime::Family) -> syn::Result<u16> {
        let value = self.value()?;
        if !family.has_weight(value) {
            let available = family
                .weights
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(syn::Error::new_spanned(
                &self.0,
                format!(
                    "`{}` does not ship weight `{value}`; available: {available}",
                    family.name
                ),
            ));
        }
        Ok(value)
    }
}

impl Parse for WeightValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for WeightValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
