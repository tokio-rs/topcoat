use proc_macro2::Span;
use syn::{
    Ident, LitInt, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// A name written as in an HTTP header, such as `theme`, `__Host-session` or
/// `Max-Age`.
///
/// Rust reads the name as identifiers and numbers separated by `-`.
pub struct HeaderName {
    pub parts: Punctuated<HeaderNamePart, Token![-]>,
}

impl HeaderName {
    /// The name as it appears in the header.
    #[must_use]
    pub fn value(&self) -> String {
        let parts: Vec<_> = self.parts.iter().map(HeaderNamePart::value).collect();
        parts.join("-")
    }

    /// The span of the first part of the name.
    #[must_use]
    pub fn span(&self) -> Span {
        match self.parts.first() {
            Some(HeaderNamePart::Ident(ident)) => ident.span(),
            Some(HeaderNamePart::Int(int)) => int.span(),
            None => Span::call_site(),
        }
    }
}

impl Parse for HeaderName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // A name cannot start with a number.
        let mut parts = Punctuated::new();
        parts.push_value(HeaderNamePart::Ident(input.call(Ident::parse_any)?));
        while input.peek(Token![-]) {
            parts.push_punct(input.parse()?);
            parts.push_value(input.parse()?);
        }
        Ok(Self { parts })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HeaderName {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        for pair in self.parts.pairs() {
            pair.value().pretty_print(printer);
            if let Some(punct) = pair.punct() {
                punct.pretty_print(printer);
            }
        }
    }
}

/// One part of a [`HeaderName`] between `-` separators.
pub enum HeaderNamePart {
    Ident(Ident),
    Int(LitInt),
}

impl HeaderNamePart {
    fn value(&self) -> String {
        match self {
            Self::Ident(ident) => ident.unraw().to_string(),
            Self::Int(int) => int.to_string(),
        }
    }
}

impl Parse for HeaderNamePart {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitInt) {
            Ok(Self::Int(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) {
            Ok(Self::Ident(input.call(Ident::parse_any)?))
        } else {
            Err(lookahead.error())
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HeaderNamePart {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Ident(ident) => ident.pretty_print(printer),
            Self::Int(int) => int.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(input: &str) -> String {
        syn::parse_str::<HeaderName>(input).unwrap().value()
    }

    #[test]
    fn joins_the_parts_with_hyphens() {
        assert_eq!(value("theme"), "theme");
        assert_eq!(value("__Host - csrf - token"), "__Host-csrf-token");
        assert_eq!(value("Max-Age"), "Max-Age");
    }

    #[test]
    fn accepts_numbers_and_keywords_after_the_first_part() {
        assert_eq!(value("session-2"), "session-2");
        assert_eq!(value("type-ref"), "type-ref");
    }

    #[test]
    fn rejects_a_leading_number() {
        assert!(syn::parse_str::<HeaderName>("2-session").is_err());
    }
}
