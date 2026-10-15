use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Ident, Path, PathSegment, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::ParseOption;

/// A path to a message, optionally followed by one of its attributes, such as
/// `msg::title` or `msg::login_input.placeholder`.
///
/// Parsing also accepts a path that ends in `::` or `.`, such as `msg::` or
/// `msg::login_input.`, so that an incomplete path can still be emitted while
/// it is being typed. Use [`is_complete`](Self::is_complete) to check for it.
pub struct MessagePath {
    /// The path to the message. Its segments may end with a trailing `::`.
    pub path: Path,
    /// The selected attribute of the message, if any.
    pub attribute: Option<MessageAttribute>,
}

impl MessagePath {
    /// Returns whether the path is complete: it does not end in `::`, and an
    /// attribute after a `.` has a name.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.path.segments.trailing_punct()
            && self
                .attribute
                .as_ref()
                .is_none_or(|attribute| attribute.name.is_some())
    }

    /// Returns the path to the message or attribute, with the `.` before an
    /// attribute replaced by `::`.
    ///
    /// An incomplete path stays incomplete, such as `msg::login_input::` for
    /// `msg::login_input.`.
    #[must_use]
    pub fn to_path(&self) -> TokenStream {
        let mut tokens = self.path.to_token_stream();
        if let Some(attribute) = &self.attribute {
            let span = attribute.dot_token.span;
            let mut joint = Punct::new(':', Spacing::Joint);
            joint.set_span(span);
            let mut alone = Punct::new(':', Spacing::Alone);
            alone.set_span(span);
            tokens.extend([TokenTree::Punct(joint), TokenTree::Punct(alone)]);
            attribute.name.to_tokens(&mut tokens);
        }
        tokens
    }
}

impl Parse for MessagePath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let leading_colon = input.parse()?;
        let mut segments = Punctuated::new();
        loop {
            segments.push_value(PathSegment::from(input.call(Ident::parse_any)?));
            if !input.peek(Token![::]) {
                break;
            }
            segments.push_punct(input.parse()?);
            if !input.peek(Ident::peek_any) {
                break;
            }
        }
        Ok(Self {
            path: Path {
                leading_colon,
                segments,
            },
            attribute: input.call(MessageAttribute::parse_option)?,
        })
    }
}

/// The attribute selected from a message, such as `.placeholder`.
pub struct MessageAttribute {
    /// The `.` before the attribute name.
    pub dot_token: Token![.],
    /// The name of the attribute, which is missing while it is being typed.
    pub name: Option<Ident>,
}

impl Parse for MessageAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            dot_token: input.parse()?,
            name: if input.peek(Ident::peek_any) {
                Some(input.call(Ident::parse_any)?)
            } else {
                None
            },
        })
    }
}

impl ParseOption for MessageAttribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![.])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<MessagePath> {
        syn::parse_str(input)
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn keeps_a_message_path_as_written() {
        let path = parse("crate::msg::title").unwrap();
        assert!(path.is_complete());
        assert_eq!(path.to_path().to_string(), tokens("crate::msg::title"));
    }

    #[test]
    fn turns_an_attribute_into_a_path_segment() {
        let path = parse("msg::login_input.placeholder").unwrap();
        assert!(path.is_complete());
        assert_eq!(
            path.to_path().to_string(),
            tokens("msg::login_input::placeholder")
        );
    }

    #[test]
    fn accepts_a_path_ending_in_double_colon() {
        let path = parse("msg::").unwrap();
        assert!(!path.is_complete());
        assert_eq!(path.to_path().to_string(), tokens("msg::"));
    }

    #[test]
    fn accepts_a_path_ending_in_a_dot() {
        let path = parse("msg::login_input.").unwrap();
        assert!(!path.is_complete());
        assert_eq!(path.to_path().to_string(), tokens("msg::login_input::"));
    }

    #[test]
    fn rejects_a_missing_path() {
        assert!(parse("").is_err());
        assert!(parse(".placeholder").is_err());
    }

    #[test]
    fn rejects_nested_attributes() {
        assert!(parse("msg::login_input.placeholder.more").is_err());
    }

    #[test]
    fn rejects_generic_arguments() {
        assert!(parse("msg::items<u8>").is_err());
    }
}
