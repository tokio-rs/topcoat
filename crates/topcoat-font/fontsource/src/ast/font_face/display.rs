use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Path, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(display);
}

/// A `display:` argument carrying the face's `font-display` strategy, written as
/// a [`FontDisplay`] variant name, e.g. `display: Swap`. Defaults to `Swap` when
/// omitted.
///
/// [`FontDisplay`]: ../../../font/enum.FontDisplay.html
pub struct Display {
    pub key: DisplayKey,
    pub colon_token: Token![:],
    pub value: DisplayValue,
}

impl Parse for Display {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Display {
    fn peek(input: ParseStream) -> bool {
        DisplayKey::peek(input)
    }
}

pub struct DisplayKey {
    pub display_kw: kw::display,
}

impl Parse for DisplayKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            display_kw: input.parse()?,
        })
    }
}

impl ParseOption for DisplayKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::display)
    }
}

/// A single display strategy, written as an enum-variant name (`Swap`) or path
/// (`FontDisplay::Swap`). Keeps the [`Path`] so its span drives validation
/// errors, and forwards verbatim when re-emitted for a nested
/// `fontsource_font_face!` call.
pub struct DisplayValue(Path);

impl DisplayValue {
    /// The trailing path segment, e.g. `FontDisplay::Swap` -> `Swap`.
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

    /// Validates the strategy and lowers it to its `FontDisplay` construction.
    ///
    /// # Errors
    ///
    /// Returns an error if the variant is not a known `font-display` strategy.
    pub fn resolve(&self) -> syn::Result<TokenStream> {
        let variant = self.variant();
        if !matches!(
            variant.to_string().as_str(),
            "Auto" | "Block" | "Swap" | "Fallback" | "Optional"
        ) {
            return Err(syn::Error::new_spanned(
                &self.0,
                format!(
                    "unknown display strategy `{variant}`; expected one of `Auto`, `Block`, \
                     `Swap`, `Fallback`, or `Optional`"
                ),
            ));
        }
        Ok(quote! { ::topcoat::font::FontDisplay::#variant })
    }
}

impl Parse for DisplayValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for DisplayValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
