use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};

/// A bracketed list `[a, b, c]`, or a bare value treated as a one-element list.
///
/// `fontsource_font!` wraps each axis's singular value (e.g.
/// [`WeightValue`](crate::fontsource::font_face::WeightValue)) in this to cross-product
/// the faces.
pub enum List<T> {
    One(T),
    Many {
        bracket_token: Bracket,
        items: Punctuated<T, Token![,]>,
    },
}

impl<T> List<T> {
    /// The written items, in source order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let (one, many) = match self {
            Self::One(item) => (Some(item), None),
            Self::Many { items, .. } => (None, Some(items.iter())),
        };
        one.into_iter().chain(many.into_iter().flatten())
    }

    /// The `compile_error!` for an empty bracketed list, which no axis accepts.
    #[must_use]
    pub fn check_empty(&self) -> Option<proc_macro2::TokenStream> {
        match self {
            Self::Many {
                bracket_token,
                items,
            } if items.is_empty() => {
                let span = bracket_token.span.join();
                Some(quote::quote_spanned! {span=>
                    ::core::compile_error!("list must not be empty");
                })
            }
            _ => None,
        }
    }
}

impl<T: Parse> Parse for List<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Bracket) {
            let content;
            let bracket_token = syn::bracketed!(content in input);
            Ok(Self::Many {
                bracket_token,
                items: Punctuated::parse_terminated(&content)?,
            })
        } else {
            Ok(Self::One(input.parse()?))
        }
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_core_grammar::pretty::PrettyPrint> topcoat_core_grammar::pretty::PrettyPrint
    for List<T>
{
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::{BreakMode, Delim};
        match self {
            Self::One(item) => item.pretty_print(printer),
            Self::Many {
                bracket_token,
                items,
            } => {
                bracket_token.pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                    items.pretty_print(printer);
                });
            }
        }
    }
}
