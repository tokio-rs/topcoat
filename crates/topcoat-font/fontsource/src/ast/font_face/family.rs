use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

use crate::runtime::{ALL, Family};

/// A Fontsource font family string literal.
pub struct FamilyName(LitStr);

impl FamilyName {
    /// Resolves the literal to a known Fontsource family.
    ///
    /// # Errors
    ///
    /// Returns an error if the name does not match any catalog family.
    pub fn resolve(&self) -> syn::Result<&'static Family> {
        let needle = self.0.value();
        ALL.iter()
            .copied()
            .find(|family| family.name == needle)
            .ok_or_else(|| {
                syn::Error::new_spanned(&self.0, format!("unknown Fontsource family {needle:?}"))
            })
    }
}

impl Parse for FamilyName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FamilyName {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
