use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};

/// A bracketed list `[a, b, c]`, or a bare value treated as a one-element list.
///
/// `fontsource_font!` wraps each axis's singular value (e.g.
/// [`WeightValue`](crate::ast::font_face::WeightValue)) in this to cross-product
/// the faces.
pub enum List<T> {
    One(T),
    Many {
        bracket_token: Bracket,
        items: Punctuated<T, Token![,]>,
    },
}

impl<T> List<T> {
    /// Validates every item with `resolve`, in source order, rejecting an empty
    /// bracketed list.
    ///
    /// # Errors
    ///
    /// Returns an error if the list is an empty bracketed list, or if `resolve`
    /// rejects any item.
    pub fn resolve_with<U>(
        &self,
        mut resolve: impl FnMut(&T) -> syn::Result<U>,
    ) -> syn::Result<Vec<U>> {
        match self {
            Self::One(item) => Ok(vec![resolve(item)?]),
            Self::Many {
                bracket_token,
                items,
            } => {
                if items.is_empty() {
                    return Err(syn::Error::new(
                        bracket_token.span.join(),
                        "list must not be empty",
                    ));
                }
                items.iter().map(resolve).collect()
            }
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
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for List<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use topcoat_pretty::{BreakMode, Delim};
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
