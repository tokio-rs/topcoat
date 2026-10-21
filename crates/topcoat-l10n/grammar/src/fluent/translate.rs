mod argument;

pub use argument::*;
use syn::parse::{Parse, ParseStream};

use crate::fluent::{DocComments, FluentKey};

/// The input of a `t!` invocation: a key, optionally followed by a comma and
/// the arguments of the message. The key and each argument can have doc
/// comments.
///
/// ```text
/// t!(greeting)
/// t!(password-field.placeholder, min = 8)
/// t!(
///     /// Shown inside the empty password field.
///     password-field.placeholder,
///     /// Minimum number of characters.
///     min = 8,
/// )
/// ```
pub struct Translate {
    /// The doc comments of the key.
    pub docs: DocComments,
    /// The message or one of its attributes.
    pub key: FluentKey,
    /// The arguments of the message.
    pub arguments: TranslateArguments,
}

impl Parse for Translate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            docs: input.parse()?,
            key: input.parse()?,
            arguments: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<Translate> {
        syn::parse_str(input)
    }

    #[test]
    fn parses_a_key_without_arguments() {
        for input in ["greeting", "greeting,"] {
            let translate = parse(input).unwrap();
            assert_eq!(translate.key.message(), "greeting");
            assert!(translate.arguments.arguments.is_empty(), "{input}");
        }
    }

    #[test]
    fn parses_doc_comments_on_the_key_and_arguments() {
        let translate = parse(
            "/// Shown inside the empty password field.
            password-field.placeholder,
            /// Minimum number of characters.
            min = 8,
            name",
        )
        .unwrap();
        assert_eq!(translate.docs.attrs.len(), 1);
        assert_eq!(translate.key.attribute().as_deref(), Some("placeholder"));
        let docs: Vec<_> = translate
            .arguments
            .arguments
            .iter()
            .map(|argument| argument.docs.attrs.len())
            .collect();
        assert_eq!(docs, [1, 0]);
    }

    #[test]
    fn rejects_arguments_without_a_comma() {
        assert!(parse("items count = 5").is_err());
    }
}
