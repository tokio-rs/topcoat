mod argument;

pub use argument::*;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::paths::topcoat_l10n;

use crate::fluent::{DocComments, FluentKey};

/// Arguments to `t!`: a message key followed by optional named arguments. The key and
/// each argument may have doc comments.
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
    /// Documentation attached to the key.
    pub docs: DocComments,
    /// The message key, including an optional attribute.
    pub key: FluentKey,
    /// Values supplied to the message.
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

/// Input forwarded by the generated `t!` macro. It starts with the hidden macro name
/// that identifies the namespace, then a semicolon and the caller's arguments.
///
/// ```text
/// __topcoat_l10n_t_25c4f22f50b921bf; password-field.placeholder, min = 8
/// ```
pub struct TranslateCall {
    /// The hidden macro name identifying this translation's namespace.
    pub namespace: Ident,
    /// The separator between the namespace and translation input.
    pub semi_token: Token![;],
    /// Arguments supplied to `t!`.
    pub translate: Translate,
}

/// Creates a `Translation` from the call record, using its namespace, key and named
/// arguments.
impl ToTokens for TranslateCall {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let namespace = self.namespace.to_string();
        let key = &self.translate.key;
        let id = key.message();
        let attribute = key.attribute().map_or_else(
            || quote! { ::core::option::Option::None },
            |attribute| quote! { ::core::option::Option::Some(#attribute) },
        );
        let arguments = &self.translate.arguments.arguments;
        let count = arguments.len();
        let names: Vec<_> = arguments
            .iter()
            .map(|argument| argument.name.value())
            .collect();
        let values = arguments.iter().map(TranslateArgument::value);
        let start = key.span().start();
        let line = u32::try_from(start.line).unwrap_or(0);
        let column = u32::try_from(start.column + 1).unwrap_or(0);
        // The values are the caller's expressions, which must not see this
        // local.
        let args = Ident::new("args", Span::mixed_site());
        tokens.extend(quote! {
            {
                let mut #args = #topcoat_l10n::FluentArgs::with_capacity(#count);
                #(#args.set(#names, #values);)*
                #topcoat_l10n::Translation::new(
                    {
                        const LEN: usize = #topcoat_l10n::RawTranslation::encoded_len(
                            #namespace,
                            #id,
                            #attribute,
                            &[#(#names),*],
                            ::core::file!(),
                        );
                        static RECORD: [u8; LEN] = #topcoat_l10n::RawTranslation::encode(
                            #namespace,
                            #id,
                            #attribute,
                            &[#(#names),*],
                            ::core::file!(),
                            #line,
                            #column,
                        );
                        &RECORD
                    },
                    #args,
                )
            }
        });
    }
}

impl Parse for TranslateCall {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            namespace: input.parse()?,
            semi_token: input.parse()?,
            translate: input.parse()?,
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
    fn parses_the_namespace_before_the_input_of_t() {
        let call: TranslateCall =
            syn::parse_str("__topcoat_l10n_t_25c4f22f50b921bf; greeting, name").unwrap();
        assert_eq!(call.namespace, "__topcoat_l10n_t_25c4f22f50b921bf");
        assert_eq!(call.translate.key.message(), "greeting");
        assert_eq!(call.translate.arguments.arguments.len(), 1);
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
