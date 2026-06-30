use syn::{
    Ident, Path, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(style);
}

/// A `style:` argument carrying the single style one face ships, e.g.
/// `style: Style::Italic`.
pub struct Style {
    pub key: StyleKey,
    pub colon_token: Token![:],
    pub value: StyleValue,
}

impl Parse for Style {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Style {
    fn peek(input: ParseStream) -> bool {
        StyleKey::peek(input)
    }
}

pub struct StyleKey {
    pub style_kw: kw::style,
}

impl Parse for StyleKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            style_kw: input.parse()?,
        })
    }
}

impl ParseOption for StyleKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::style)
    }
}

/// A single style, written as an enum-variant path (`Style::Normal`). Keeps the
/// [`Path`] so its span drives validation errors.
pub struct StyleValue(Path);

impl StyleValue {
    /// The trailing path segment, e.g. `Style::Normal` -> `Normal`.
    #[must_use]
    pub fn variant(&self) -> &Ident {
        &self
            .0
            .segments
            .last()
            .expect("a parsed path has at least one segment")
            .ident
    }

    /// Validates the style against the family's catalog.
    pub fn resolve(&self, family: &runtime::Family) -> syn::Result<runtime::Style> {
        let variant = self.variant();
        let style = match variant.to_string().as_str() {
            "Normal" => runtime::Style::Normal,
            "Italic" => runtime::Style::Italic,
            _ => {
                return Err(syn::Error::new_spanned(
                    &self.0,
                    format!(
                        "unknown style `{variant}`; expected `Style::Normal` or `Style::Italic`"
                    ),
                ));
            }
        };
        if !family.has_style(style) {
            return Err(syn::Error::new_spanned(
                &self.0,
                format!("`{}` does not ship the `{variant}` style", family.name),
            ));
        }
        Ok(style)
    }
}

impl Parse for StyleValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}
