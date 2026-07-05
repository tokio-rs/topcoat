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

use crate::ast::font_face::{Display, FamilyName, Host};

/// One `fontsource_font!` invocation: a family and the axes to cross-product
/// into faces.
///
/// Holds the parsed descriptors. An omitted `weight` or `style` expands to every
/// value the family ships; an omitted `subset` expands to the family's default
/// subset only. `display` applies to every face and defaults to `swap`. The
/// cross product is emitted as `fontsource_font_face!` calls, each of which
/// verifies its own combination against the catalog.
pub struct FontsourceFont {
    pub family: FamilyName,
    pub weight: Option<Weight>,
    pub style: Option<Style>,
    pub subset: Option<Subset>,
    pub display: Option<Display>,
    pub host: Option<Host>,
}

impl ToTokens for FontsourceFont {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let family_path = &self.family;
        let family_ident = self.family.ident();
        let family = self.family.family();

        // Empty bracketed lists are the only mistake the nested calls cannot
        // report themselves, because they cross-product into no calls at all.
        let mut checks = Vec::new();
        if let Some(weight) = &self.weight {
            checks.extend(weight.value.check_empty());
        }
        if let Some(style) = &self.style {
            checks.extend(style.value.check_empty());
        }
        if let Some(subset) = &self.subset {
            checks.extend(subset.value.check_empty());
        }

        // Weight and style token lists: written values verbatim, otherwise
        // every value the family ships. `None` when the family is unknown and
        // the axis is omitted: the emitted family path reports the former.
        let weights: Option<Vec<TokenStream>> = match &self.weight {
            Some(weight) => Some(weight.value.iter().map(ToTokens::to_token_stream).collect()),
            None => family.map(|family| {
                family
                    .weights
                    .iter()
                    .map(|&weight| Literal::u16_unsuffixed(weight).to_token_stream())
                    .collect()
            }),
        };
        let styles: Option<Vec<TokenStream>> = match &self.style {
            Some(style) => Some(
                style
                    .value
                    .iter()
                    .map(|style| style.ident().to_token_stream())
                    .collect(),
            ),
            None => family.map(|family| {
                family
                    .styles
                    .iter()
                    .map(|style| {
                        Ident::new(&format!("{style:?}"), Span::call_site()).to_token_stream()
                    })
                    .collect()
            }),
        };
        // An omitted subset is not passed on, leaving each face on the
        // family's default subset.
        let subsets: Vec<Option<TokenStream>> = match &self.subset {
            Some(subset) => subset
                .value
                .iter()
                .map(|subset| Some(subset.ident().to_token_stream()))
                .collect(),
            None => vec![None],
        };

        let display = self.display.as_ref().map(|display| {
            let ident = display.value.ident();
            quote! { , display: #ident }
        });
        let host = self.host.as_ref().map(|host| {
            let ident = host.value.ident();
            quote! { , host: #ident }
        });

        let faces = match (weights, styles) {
            (Some(weights), Some(styles)) => {
                let mut faces = Vec::new();
                for weight in &weights {
                    for style in &styles {
                        for subset in &subsets {
                            let subset = subset.as_ref().map(|subset| quote! { , subset: #subset });
                            faces.push(quote! {
                                ::topcoat::font::fontsource::fontsource_font_face!(
                                    #family_ident,
                                    weight: #weight,
                                    style: #style
                                    #subset
                                    #display
                                    #host
                                )
                            });
                        }
                    }
                }
                faces
            }
            _ => Vec::new(),
        };

        // With no faces the compiler is already reporting the cause (an
        // unknown family or an empty axis), so emit a well-typed empty list.
        let faces = if faces.is_empty() {
            quote! { ::std::vec::Vec::<::topcoat::font::FontFace>::new() }
        } else {
            quote! { ::std::vec![#(#faces),*] }
        };

        quote! {{
            const FAMILY: &'static ::topcoat::font::fontsource::Family = &#family_path;
            #(#checks)*
            ::topcoat::font::font!(FAMILY.name, #faces)
        }}
        .to_tokens(tokens);
    }
}

impl Parse for FontsourceFont {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let family = input.parse()?;

        let mut weight = None;
        let mut style = None;
        let mut subset = None;
        let mut display = None;
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
            } else if Display::peek(input) {
                if display.is_some() {
                    return Err(input.error("duplicate `display`"));
                }
                display = Some(input.parse()?);
            } else if Host::peek(input) {
                if host.is_some() {
                    return Err(input.error("duplicate `host`"));
                }
                host = Some(input.parse()?);
            } else {
                return Err(
                    input.error("expected `weight`, `style`, `subset`, `display`, or `host`")
                );
            }
        }

        Ok(Self {
            family,
            weight,
            style,
            subset,
            display,
            host,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontsourceFont {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        // The axes live in fixed fields, so recover their written order from
        // their spans to keep the output faithful and idempotent. Every axis is
        // optional.
        let mut descriptors: Vec<(proc_macro2::LineColumn, &dyn topcoat_pretty::PrettyPrint)> =
            Vec::new();
        if let Some(weight) = &self.weight {
            descriptors.push((weight.key.weight_kw.span().start(), weight));
        }
        if let Some(style) = &self.style {
            descriptors.push((style.key.style_kw.span().start(), style));
        }
        if let Some(subset) = &self.subset {
            descriptors.push((subset.key.subset_kw.span().start(), subset));
        }
        if let Some(display) = &self.display {
            descriptors.push((display.key.display_kw.span().start(), display));
        }
        if let Some(host) = &self.host {
            descriptors.push((host.key.host_kw.span().start(), host));
        }
        descriptors.sort_by_key(|(position, _)| (position.line, position.column));

        crate::ast::font_face::pretty_print_arguments(printer, &self.family, &descriptors);
    }
}
