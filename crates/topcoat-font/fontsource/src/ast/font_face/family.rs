use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

use crate::{ALL, Family};

/// The leading family argument: a display name or Fontsource id (`"Roboto"`,
/// `"roboto"`), resolved against the vendored catalog while parsing.
pub struct FamilyName(&'static Family);

impl FamilyName {
    /// The resolved catalog family.
    pub fn family(&self) -> &'static Family {
        self.0
    }
}

impl Parse for FamilyName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        let needle = lit.value();
        ALL.iter()
            .copied()
            .find(|family| {
                family.name.eq_ignore_ascii_case(&needle) || family.id.eq_ignore_ascii_case(&needle)
            })
            .map(Self)
            .ok_or_else(|| {
                syn::Error::new_spanned(&lit, format!("unknown Fontsource family {needle:?}"))
            })
    }
}
