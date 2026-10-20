mod argument;
mod message_path;

pub use argument::*;
pub use message_path::*;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Ident,
    parse::{Parse, ParseStream},
};

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

/// Expands to a call of the message module's `format` with its `Args`, such
/// as `msg::items::format(msg::items::Args { count: Into::into(5) })`. A
/// missing or unknown argument is reported as a missing or unknown field of
/// `Args`.
///
/// Input that is still being typed expands to the same shape, with the path
/// and values as written, so that rust-analyzer can complete them.
impl ToTokens for Translate {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = self.message.to_path();
        // Errors about the module, such as a message without a value having no
        // `format`, point at the end of the path.
        let span = path
            .clone()
            .into_iter()
            .last()
            .map_or_else(Span::call_site, |token| token.span());
        let format = Ident::new("format", span);
        let args = Ident::new("Args", span);
        let fields = self.arguments.arguments.iter().map(|argument| {
            let name = &argument.name;
            let value = argument.value();
            // Errors about a value that does not convert point at the value.
            let span = value
                .clone()
                .into_iter()
                .next()
                .map_or_else(|| name.span(), |token| token.span());
            quote_spanned! {span=> #name: ::core::convert::Into::into(#value) }
        });
        tokens.extend(quote! {
            #path::#format(#path::#args { #(#fields),* })
        });
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

    fn expand(input: &str) -> String {
        parse(input).unwrap().to_token_stream().to_string()
    }

    #[test]
    fn expands_to_a_call_of_format() {
        assert_eq!(
            expand("msg::items, count = n + 1, name"),
            tokens(
                "msg::items::format(msg::items::Args { \
                 count: ::core::convert::Into::into(n + 1), \
                 name: ::core::convert::Into::into(name) \
                 })"
            )
        );
    }

    #[test]
    fn expands_an_attribute_to_its_module() {
        assert_eq!(
            expand("crate::msg::login_input.placeholder"),
            tokens(
                "crate::msg::login_input::placeholder::format(\
                 crate::msg::login_input::placeholder::Args {})"
            )
        );
    }

    #[test]
    fn keeps_an_incomplete_path_in_place() {
        assert_eq!(expand("msg::"), tokens("msg:: ::format(msg:: ::Args {})"));
        assert_eq!(
            expand("msg::login_input."),
            tokens("msg::login_input:: ::format(msg::login_input:: ::Args {})")
        );
    }

    #[test]
    fn keeps_an_argument_being_typed_in_place() {
        let translate = parse("msg::items, count =, name").unwrap();
        let error = translate.arguments.arguments[0].value();
        assert!(error.to_string().contains("compile_error"), "{error}");
        assert_eq!(
            translate.to_token_stream().to_string(),
            tokens(&format!(
                "msg::items::format(msg::items::Args {{ \
                 count: ::core::convert::Into::into({error}), \
                 name: ::core::convert::Into::into(name) \
                 }})"
            ))
        );
    }
}
