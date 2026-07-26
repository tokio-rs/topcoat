mod field_value;
mod mail_field;

pub use field_value::*;
pub use mail_field::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Token;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use topcoat_core_grammar::paths::{topcoat_error, topcoat_mail};

/// The parsed body of a `mail!` invocation: `name: value` fields in any
/// order, at most once each. Lowers to an awaited `async` block that
/// assembles a [`Mail`](topcoat_mail::Mail) through
/// [`MailBuilder`](topcoat_mail::MailBuilder) and produces a `Result<Mail>`.
pub struct Mail {
    pub fields: Punctuated<MailField, Token![,]>,
}

impl Parse for Mail {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fields: Punctuated<MailField, Token![,]> = Punctuated::parse_terminated(input)?;

        let mut seen = Vec::new();
        for field in &fields {
            let name = field.name();
            if seen.contains(&name) {
                return Err(syn::Error::new(
                    field.name.span(),
                    format!("duplicate `{name}` field"),
                ));
            }
            seen.push(name);
        }

        Ok(Self { fields })
    }
}

impl ToTokens for Mail {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let calls = self.fields.iter().map(MailField::builder_calls);
        quote! {
            async {
                let __builder = #topcoat_mail::Mail::builder();
                #(#calls)*
                ::core::result::Result::<#topcoat_mail::Mail, #topcoat_error::Error>::Ok(
                    __builder.build(),
                )
            }
            .await
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Mail {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.fields.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Mail {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Mail>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn expand(source: &str) -> String {
        parse(source).to_token_stream().to_string()
    }

    #[test]
    fn empty_body_builds_a_default_mail() {
        let tokens = expand("");
        assert!(tokens.contains("Mail :: builder ()"), "{tokens}");
        assert!(tokens.contains(". build ()"), "{tokens}");
    }

    #[test]
    fn expands_to_an_awaited_async_block() {
        let tokens = expand(r#"subject: "Hello""#);
        assert!(tokens.starts_with("async"), "{tokens}");
        assert!(tokens.ends_with(". await"), "{tokens}");
    }

    #[test]
    fn parses_every_field() {
        let mail = parse(
            r#"
            from: sender,
            to: [a, b],
            cc: c,
            bcc: [d],
            reply_to: replies,
            subject: "Hello",
            html: { <p>"Hi"</p> },
            text: "Hi",
            attachments: [invoice],
            headers: [("List-Unsubscribe", "<mailto:stop@example.com>")],
            in_reply_to: earlier,
            references: earlier,
            date: now,
            message_id: id,
            "#,
        );
        assert_eq!(mail.fields.len(), 14);
    }

    #[test]
    fn emits_builder_calls_in_written_order() {
        let tokens = expand(r#"subject: "Hello", from: sender"#);
        let subject = tokens.find(". subject").unwrap();
        let from = tokens.find(". from").unwrap();
        assert!(subject < from, "{tokens}");
    }

    #[test]
    fn from_converts_through_try_from() {
        let tokens = expand(r#"from: "ada@example.com""#);
        assert!(tokens.contains("Mailbox :: try_from"), "{tokens}");
    }

    #[test]
    fn recipients_convert_through_try_into_mailboxes() {
        let tokens = expand(r#"to: [bob, grace], cc: "carol@example.com""#);
        assert_eq!(
            tokens
                .matches("TryIntoMailboxes :: try_into_mailboxes")
                .count(),
            2,
            "{tokens}"
        );
        assert!(tokens.contains(". to ("), "{tokens}");
        assert!(tokens.contains(". cc ("), "{tokens}");
    }

    #[test]
    fn non_mailbox_values_pass_through() {
        let tokens = expand(r#"subject: "Hello", text: "Hi""#);
        assert!(!tokens.contains("try_from"), "{tokens}");
        assert!(!tokens.contains("try_into_mailboxes"), "{tokens}");
    }

    #[test]
    fn attachments_convert_through_into_attachments() {
        let tokens = expand("attachments: [a, b]");
        assert!(
            tokens.contains("IntoAttachments :: into_attachments"),
            "{tokens}"
        );
        assert!(tokens.contains(". attachments ("), "{tokens}");
    }

    #[test]
    fn headers_convert_through_into_headers() {
        let tokens = expand(r#"headers: [("X-One", "1")]"#);
        assert!(tokens.contains("IntoHeaders :: into_headers"), "{tokens}");
        assert!(tokens.contains(". headers ("), "{tokens}");
    }

    #[test]
    fn html_field_expands_the_view_body() {
        let tokens = expand(r#"html: { <p>"Hi"</p> }"#);
        assert!(tokens.contains(". html ("), "{tokens}");
        assert!(tokens.contains("<p>Hi</p>"), "{tokens}");
    }

    #[test]
    fn html_field_accepts_a_leading_cx_argument() {
        let tokens = expand("html: { cx => <p>(greeting)</p> }");
        assert!(tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn html_field_accepts_a_view_expression() {
        let tokens = expand("html: prebuilt");
        assert!(tokens.contains(". html (prebuilt)"), "{tokens}");
    }

    #[test]
    fn unknown_field_lowers_to_a_builder_call() {
        // The compiler reports the unknown name as a missing `MailBuilder`
        // method, which also lets rust-analyzer suggest the builder's
        // methods at the name.
        let tokens = expand("sender: a");
        assert!(tokens.contains(". sender (a)"), "{tokens}");
    }

    #[test]
    fn field_without_a_value_lowers_to_a_bare_access() {
        let tokens = expand("subj");
        assert!(tokens.contains("__builder . subj ;"), "{tokens}");
    }

    #[test]
    fn field_with_a_dangling_colon_lowers_to_a_bare_access() {
        let tokens = expand("subject: , to: a");
        assert!(tokens.contains("__builder . subject ;"), "{tokens}");
        assert!(tokens.contains(". to ("), "{tokens}");
    }

    #[test]
    fn keyword_field_name_is_escaped() {
        // `ref` on the way to `references` is a keyword; the expansion
        // escapes it to stay valid Rust.
        let tokens = expand("ref");
        assert!(tokens.contains("__builder . r#ref ;"), "{tokens}");
    }

    #[test]
    fn rejects_a_duplicate_field() {
        assert!(parse_err(r#"subject: "a", subject: "b""#).contains("duplicate `subject` field"));
    }

    #[test]
    fn rejects_a_duplicate_additive_field() {
        assert!(parse_err("to: [a], to: [b]").contains("duplicate `to` field"));
    }
}
