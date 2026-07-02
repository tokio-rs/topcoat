use syn::{
    Ident, Path, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(subset);
}

/// A `subset:` argument carrying the single subset one face ships, e.g.
/// `subset: Subset::Latin`.
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

/// A single subset, written as an enum-variant path (`Subset::Latin`). Keeps the
/// [`Path`] so its span drives validation errors.
pub struct SubsetValue(Path);

impl SubsetValue {
    /// The trailing path segment, e.g. `Subset::LatinExt` -> `LatinExt`.
    ///
    /// # Panics
    ///
    /// Panics if the parsed path has no segments, which a successfully parsed
    /// [`Path`] never does.
    #[must_use]
    pub fn variant(&self) -> &Ident {
        &self
            .0
            .segments
            .last()
            .expect("a parsed path has at least one segment")
            .ident
    }

    /// Validates the subset against the family's catalog, matching on the enum
    /// variant name.
    ///
    /// # Errors
    ///
    /// Returns an error if the family does not ship the named subset.
    pub fn resolve(&self, family: &runtime::Family) -> syn::Result<runtime::Subset> {
        let variant = self.variant().to_string();
        family
            .subsets
            .iter()
            .copied()
            .find(|subset| format!("{subset:?}") == variant)
            .ok_or_else(|| {
                let available = family
                    .subsets
                    .iter()
                    .map(|subset| format!("{subset:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                syn::Error::new_spanned(
                    &self.0,
                    format!(
                        "`{}` does not ship the `{variant}` subset; available: {available}",
                        family.name,
                    ),
                )
            })
    }
}

impl Parse for SubsetValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for SubsetValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
