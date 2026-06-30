mod list;
mod style;
mod subset;
mod weight;

pub use list::*;
pub use style::*;
pub use subset::*;
pub use weight::*;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::{
    ast::font_face::{FamilyName, Host},
    runtime,
};

/// One `fontsource_font!` invocation: a family and the axes to cross-product
/// into faces.
///
/// Holds the parsed descriptors. An omitted `weight` or `style` expands to every
/// value the family ships; an omitted `subset` expands to the family's default
/// subset only. Each axis is validated against the catalog when lowering.
pub struct FontsourceFont {
    pub family: FamilyName,
    pub weight: Option<Weight>,
    pub style: Option<Style>,
    pub subset: Option<Subset>,
    pub host: Option<Host>,
}

impl FontsourceFont {
    /// Validates the axes, then lowers to a `const` [`Font`] whose faces are the
    /// cross product, emitted as `fontsource_font_face!` calls.
    fn lower(&self) -> syn::Result<TokenStream> {
        let family = self.family.resolve()?;
        let weights = match &self.weight {
            Some(weight) => weight.value.resolve_with(|value| value.resolve(family))?,
            None => family.weights.to_vec(),
        };
        let styles = match &self.style {
            Some(style) => style.value.resolve_with(|value| value.resolve(family))?,
            None => family.styles.to_vec(),
        };
        let subsets = match &self.subset {
            Some(subset) => subset.value.resolve_with(|value| value.resolve(family))?,
            None => vec![family.default_subset],
        };

        let name = family.name;
        let host = if self.host.is_some() {
            quote! { , host: asset }
        } else {
            quote! {}
        };

        let mut faces = Vec::new();
        for &weight in &weights {
            for &style in &styles {
                for &subset in &subsets {
                    let weight = Literal::u16_unsuffixed(weight);
                    let style = match style {
                        runtime::Style::Normal => Ident::new("Normal", Span::call_site()),
                        runtime::Style::Italic => Ident::new("Italic", Span::call_site()),
                    };
                    let subset = Ident::new(&format!("{subset:?}"), Span::call_site());
                    faces.push(quote! {
                        ::topcoat::font::fontsource::fontsource_font_face!(
                            #name,
                            weight: #weight,
                            style: ::topcoat::font::fontsource::Style::#style,
                            subset: ::topcoat::font::fontsource::Subset::#subset
                            #host
                        )
                    });
                }
            }
        }

        Ok(quote! {{
            const FONT: ::topcoat::font::Font = ::topcoat::font::Font::const_new(
                #name,
                ::topcoat::font::FontFaces::const_new(const { &[#(#faces),*] }),
            );
            ::topcoat::font::__register_font!(FONT);
            FONT
        }})
    }
}

impl Parse for FontsourceFont {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let family = input.parse()?;

        let mut weight = None;
        let mut style = None;
        let mut subset = None;
        let mut host = None;

        while !input.is_empty() {
            let _: Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }

            if Weight::peek(input) {
                if weight.is_some() {
                    return Err(input.error("duplicate `weight`"));
                }
                weight = Some(input.parse()?);
            } else if Style::peek(input) {
                if style.is_some() {
                    return Err(input.error("duplicate `style`"));
                }
                style = Some(input.parse()?);
            } else if Subset::peek(input) {
                if subset.is_some() {
                    return Err(input.error("duplicate `subset`"));
                }
                subset = Some(input.parse()?);
            } else if Host::peek(input) {
                if host.is_some() {
                    return Err(input.error("duplicate `host`"));
                }
                host = Some(input.parse()?);
            } else {
                return Err(input.error("expected `weight`, `style`, `subset`, or `host`"));
            }
        }

        Ok(Self {
            family,
            weight,
            style,
            subset,
            host,
        })
    }
}

impl ToTokens for FontsourceFont {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.lower() {
            Ok(font) => font.to_tokens(tokens),
            Err(error) => error.to_compile_error().to_tokens(tokens),
        }
    }
}
