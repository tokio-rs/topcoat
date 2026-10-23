use proc_macro2::Span;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::ParseOption;

use crate::fluent::FluentIdent;

/// A Fluent message, optionally followed by one of its attributes.
///
/// The key is written as identifiers, or as a single string literal that
/// contains both.
///
/// ```text
/// password-field
/// password-field.placeholder
/// "item-1e".label
/// "item-1e.label"
/// ```
pub enum FluentKey {
    /// The key written as identifiers.
    Idents {
        /// The ID of the message.
        message: FluentIdent,
        /// The attribute of the message, if any.
        attribute: Option<FluentKeyAttribute>,
    },
    /// The key written as a single string literal.
    Str(LitStr),
}

impl FluentKey {
    /// Returns the ID of the message.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Idents { message, .. } => message.value(),
            Self::Str(lit) => {
                let mut value = lit.value();
                if let Some(dot) = value.find('.') {
                    value.truncate(dot);
                }
                value
            }
        }
    }

    /// Returns the span of the message ID, where the key starts.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Idents { message, .. } => message.span(),
            Self::Str(lit) => lit.span(),
        }
    }

    /// Returns the name of the attribute, if any.
    #[must_use]
    pub fn attribute(&self) -> Option<String> {
        match self {
            Self::Idents { attribute, .. } => {
                attribute.as_ref().map(|attribute| attribute.name.value())
            }
            Self::Str(lit) => lit
                .value()
                .split_once('.')
                .map(|(_, attribute)| attribute.to_owned()),
        }
    }
}

impl Parse for FluentKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // A string literal followed by a `.` is only the message ID.
        if input.peek(LitStr) && !input.peek2(Token![.]) {
            let lit: LitStr = input.parse()?;
            FluentIdent::check_str_suffix(&lit)?;
            let value = lit.value();
            match value.find('.') {
                Some(dot) => {
                    FluentIdent::check_str_part(&lit, &value, 0..dot)?;
                    FluentIdent::check_str_part(&lit, &value, dot + 1..value.len())?;
                }
                None => FluentIdent::check_str_part(&lit, &value, 0..value.len())?,
            }
            return Ok(Self::Str(lit));
        }

        let key = Self::Idents {
            message: input.parse()?,
            attribute: input.call(FluentKeyAttribute::parse_option)?,
        };
        if input.peek(Token![.]) {
            return Err(input.error("a Fluent attribute cannot have attributes"));
        }
        Ok(key)
    }
}

/// The attribute selected from a message, such as `.placeholder`.
pub struct FluentKeyAttribute {
    /// The `.` before the attribute name.
    pub dot_token: Token![.],
    /// The name of the attribute.
    pub name: FluentIdent,
}

impl Parse for FluentKeyAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            dot_token: input.parse()?,
            name: input.parse()?,
        })
    }
}

impl ParseOption for FluentKeyAttribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![.])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<FluentKey> {
        syn::parse_str(input)
    }

    /// The message ID and attribute name of a key.
    fn key(input: &str) -> (String, Option<String>) {
        let key = parse(input).unwrap();
        (key.message(), key.attribute())
    }

    #[test]
    fn parses_a_message() {
        for input in ["password-field", r#""password-field""#] {
            assert_eq!(key(input), ("password-field".to_owned(), None), "{input}");
        }
    }

    #[test]
    fn parses_an_attribute() {
        for input in [
            "password-field.placeholder",
            "password-field . placeholder",
            r#""password-field".placeholder"#,
            r#"password-field."placeholder""#,
            r#""password-field.placeholder""#,
        ] {
            assert_eq!(
                key(input),
                ("password-field".to_owned(), Some("placeholder".to_owned())),
                "{input}"
            );
        }
    }

    #[test]
    fn parses_a_number_before_the_attribute() {
        assert_eq!(
            key("step-2.label"),
            ("step-2".to_owned(), Some("label".to_owned()))
        );
    }

    #[test]
    fn rejects_nested_attributes() {
        for input in ["a.b.c", r#""a.b.c""#, r#""a".b.c"#] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_a_missing_part() {
        for input in ["", "a.", ".b", r#""a.""#, r#"".b""#, r#""""#] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_invalid_identifiers_in_a_string() {
        for input in [r#""a b.c""#, r#""a.b c""#, r#""a.2b""#, r#""a.b"x"#] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn points_at_the_invalid_character_of_the_attribute() {
        let error = parse(r#""ab.c d""#).err().unwrap();
        // The space follows the quote, the message, the dot and one letter.
        assert_eq!(error.span().start().column, 5);
    }
}
