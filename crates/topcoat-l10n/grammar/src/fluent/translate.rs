mod argument;
mod message_path;

pub use argument::*;
pub use message_path::*;
use syn::parse::{Parse, ParseStream};

/// The input of a `t!` invocation: a message, optionally followed by a comma
/// and the message's arguments.
///
/// ```text
/// t!(msg::title)
/// t!(msg::items, count = 5)
/// t!(msg::login_input.placeholder, name)
/// ```
pub struct Translate {
    /// The message or one of its attributes.
    pub message: MessagePath,
    /// The arguments of the message.
    pub arguments: TranslateArguments,
}

impl Parse for Translate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            message: input.parse()?,
            arguments: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;

    use super::*;

    fn parse(input: &str) -> syn::Result<Translate> {
        syn::parse_str(input)
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn parses_a_message_without_arguments() {
        let translate = parse("msg::title").unwrap();
        assert_eq!(
            translate.message.to_path().to_string(),
            tokens("msg::title")
        );
        assert!(translate.arguments.arguments.is_empty());
    }

    #[test]
    fn parses_arguments_after_an_attribute() {
        let translate = parse("msg::login_input.placeholder, name = user").unwrap();
        assert_eq!(
            translate.message.to_path().to_string(),
            tokens("msg::login_input::placeholder")
        );
        assert_eq!(translate.arguments.arguments.len(), 1);
    }

    #[test]
    fn rejects_arguments_without_a_comma() {
        assert!(parse("msg::items count = 5").is_err());
    }
}
