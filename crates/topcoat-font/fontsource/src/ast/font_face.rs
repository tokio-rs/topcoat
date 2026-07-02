mod display;
mod family;
mod host;
mod style;
mod subset;
mod weight;

pub use display::*;
pub use family::*;
pub use host::*;
pub use style::*;
pub use subset::*;
pub use weight::*;

use proc_macro2::{Literal, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::runtime;

/// One `fontsource_font_face!` invocation: a single Fontsource face served from
/// the jsDelivr CDN, or, with `host: Asset`, a bundled copy.
///
/// Holds the parsed descriptors; `weight` and `style` are required, `subset`
/// defaults to the family's default subset, and `display` defaults to `swap`.
/// Each value is validated against the catalog when lowering to tokens.
pub struct FontsourceFontFace {
    pub family: FamilyName,
    pub weight: Weight,
    pub style: Style,
    pub subset: Option<Subset>,
    pub display: Option<Display>,
    pub host: Option<Host>,
}

impl FontsourceFontFace {
    /// Validates every descriptor against the family and lowers the face to its
    /// [`FontFace`] construction.
    fn lower(&self) -> syn::Result<TokenStream> {
        let family = self.family.resolve()?;
        let weight = self.weight.value.resolve(family)?;
        let style = self.style.value.resolve(family)?;
        let subset = match &self.subset {
            Some(subset) => subset.value.resolve(family)?,
            None => family.default_subset,
        };
        let display = if let Some(display) = &self.display {
            display.value.resolve()?
        } else {
            quote! { ::topcoat::font::FontDisplay::Swap }
        };
        let host = self
            .host
            .as_ref()
            .map_or(runtime::Host::JsDelivr, Host::host);
        let host_autocomplete = self.host.as_ref().map(|host| quote! { let _ = #host; });

        let name = family.name;
        let url = format!(
            "https://cdn.jsdelivr.net/fontsource/fonts/{}@latest/{}-{weight}-{}.woff2",
            family.id,
            subset.as_str(),
            style.as_str(),
        );
        let weight = Literal::u16_unsuffixed(weight);
        let style = match style {
            runtime::Style::Normal => quote! { ::topcoat::font::FontStyle::Normal },
            runtime::Style::Italic => quote! { ::topcoat::font::FontStyle::Italic },
        };

        // A bundled asset when self-hosted, otherwise the CDN URL verbatim.
        let src = match host {
            #[cfg(feature = "asset")]
            runtime::Host::Asset => {
                quote! {{
                    const ASSET: ::topcoat::asset::Asset = ::topcoat::asset::asset!(#url);
                    ::topcoat::font::FontSources::new(::std::vec![
                        ::topcoat::font::FontSource::url(
                            ASSET,
                            ::core::option::Option::Some(::topcoat::font::FontFormat::Woff2),
                            ::core::option::Option::None,
                        )
                    ])
                }}
            }
            runtime::Host::JsDelivr => {
                quote! {
                    ::topcoat::font::FontSources::new(::std::vec![
                        ::topcoat::font::FontSource::url(
                            #url,
                            ::core::option::Option::Some(::topcoat::font::FontFormat::Woff2),
                            ::core::option::Option::None,
                        )
                    ])
                }
            }
        };

        // The subset's `unicode-range`, when the catalog ships one for it.
        let unicode_range = family.unicode_range(subset).map(|ranges| {
            let entries = ranges.iter().map(|range| {
                let start = u32::from(range.start());
                let end = u32::from(range.end());
                quote! { ::topcoat::font::UnicodeRange::from_u32(#start, #end) }
            });
            quote! {
                .with_unicode_range(::topcoat::font::UnicodeRanges::new(const { &[#(#entries),*] }))
            }
        });

        Ok(quote! {{
            #host_autocomplete
            ::topcoat::font::FontFace::new(#name, #src)
                .with_weight(::topcoat::font::FontWeightRange::from_u16(#weight, #weight))
                .with_style(#style)
                .with_display(#display)
                #unicode_range
        }})
    }
}

impl Parse for FontsourceFontFace {
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
            weight: weight
                .ok_or_else(|| input.error("`fontsource_font_face!` requires a `weight`"))?,
            style: style
                .ok_or_else(|| input.error("`fontsource_font_face!` requires a `style`"))?,
            subset,
            display,
            host,
        })
    }
}

impl ToTokens for FontsourceFontFace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.lower() {
            Ok(face) => face.to_tokens(tokens),
            Err(error) => error.to_compile_error().to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontsourceFontFace {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        // The descriptors live in fixed fields, so recover their written order
        // from their spans to keep the output faithful and idempotent. `weight`
        // and `style` are required; the rest are optional.
        let mut descriptors: Vec<(proc_macro2::LineColumn, &dyn topcoat_pretty::PrettyPrint)> = vec![
            (self.weight.key.weight_kw.span().start(), &self.weight),
            (self.style.key.style_kw.span().start(), &self.style),
        ];
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

        pretty_print_arguments(printer, &self.family, &descriptors);
    }
}

/// Lays out a Fontsource macro invocation as a comma-separated argument list: the
/// family name literal followed by each descriptor in written order. The list
/// stays on one line when it fits and otherwise breaks with one argument per line
/// and a trailing comma.
#[cfg(feature = "pretty")]
pub(crate) fn pretty_print_arguments(
    printer: &mut topcoat_pretty::Printer<'_>,
    family: &FamilyName,
    descriptors: &[(proc_macro2::LineColumn, &dyn topcoat_pretty::PrettyPrint)],
) {
    use topcoat_pretty::{PrettyPrint, TextMode};

    family.pretty_print(printer);
    for (_, descriptor) in descriptors {
        ",".pretty_print(printer);
        printer.scan_same_line_trivia();
        printer.scan_break();
        " ".pretty_print(printer);
        printer.scan_trivia(true, true);
        descriptor.pretty_print(printer);
    }
    // A trailing comma is only rendered when the argument list breaks across
    // lines.
    printer.scan_text(",".into(), TextMode::Break);
    printer.advance_cursor(",");
}
