use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

use crate::runtime::{ALL, Family};

/// A Fontsource font family string literal.
pub struct FamilyName(LitStr);

impl FamilyName {
    #[must_use]
    pub fn resolve(&self) -> syn::Result<&'static Family> {
        let needle = self.0.value();
        ALL.iter()
            .copied()
            .find(|family| family.name == &needle)
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
