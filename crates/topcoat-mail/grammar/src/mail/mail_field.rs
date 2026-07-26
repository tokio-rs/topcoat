use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::paths::topcoat_mail;

use crate::mail::FieldValue;

/// A single `name: value` entry in a `mail!` body.
///
/// The name is any identifier, not just the known field names: an
/// unrecognized or partially typed name still lowers to a call on
/// [`MailBuilder`](topcoat_mail::MailBuilder), so the compiler reports it as
/// a missing method there and rust-analyzer can complete the builder's
/// methods at the name. The colon and value are optional for the same
/// reason; a field still being typed lowers to a bare builder access.
pub struct MailField {
    pub name: Ident,
    pub colon_token: Option<Token![:]>,
    pub value: Option<FieldValue>,
}

impl Parse for MailField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Keywords are accepted as names so that a partially typed name that
        // happens to be one (`ref` on the way to `references`) keeps the
        // body parsing.
        let name = input.call(Ident::parse_any)?;
        let colon_token: Option<Token![:]> = input.parse()?;
        let value = match colon_token {
            Some(_) if !input.is_empty() && !input.peek(Token![,]) => {
                Some(FieldValue::parse_named(&name, input)?)
            }
            _ => None,
        };
        Ok(Self {
            name,
            colon_token,
            value,
        })
    }
}

impl MailField {
    /// The field name as written, without a raw-identifier prefix.
    #[must_use]
    pub fn name(&self) -> String {
        self.name.unraw().to_string()
    }

    /// The builder calls this field lowers to inside the generated block.
    pub(crate) fn builder_calls(&self) -> TokenStream {
        let method = self.method();
        let Some(value) = &self.value else {
            // No value yet: a bare access keeps the expansion valid while
            // the field is typed, so diagnostics and completion see the
            // builder.
            return quote! { let __builder = __builder.#method; };
        };

        match value {
            FieldValue::Html(html) => {
                let view = &html.view;
                quote! {
                    let __html = #view;
                    let __builder = __builder.#method(__html?);
                }
            }
            FieldValue::Expr(value) => match self.name().as_str() {
                "to" | "cc" | "bcc" | "reply_to" => quote! {
                    let __builder = __builder
                        .#method(#topcoat_mail::TryIntoMailboxes::try_into_mailboxes(#value)?);
                },
                "from" => quote! {
                    let __builder = __builder.#method(#topcoat_mail::Mailbox::try_from(#value)?);
                },
                "attachments" => quote! {
                    let __builder = __builder
                        .#method(#topcoat_mail::IntoAttachments::into_attachments(#value));
                },
                "headers" => quote! {
                    let __builder = __builder
                        .#method(#topcoat_mail::IntoHeaders::into_headers(#value));
                },
                _ => quote! { let __builder = __builder.#method(#value); },
            },
        }
    }

    /// The builder method the field lowers to: the name as written, with
    /// keywords escaped to raw identifiers to keep the expansion parseable.
    fn method(&self) -> Ident {
        match syn::parse_str::<Ident>(&self.name()) {
            Ok(_) => self.name.clone(),
            Err(_) => Ident::new_raw(&self.name(), self.name.span()),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for MailField {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.name.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        if let Some(value) = &self.value {
            " ".pretty_print(printer);
            value.pretty_print(printer);
        }
    }
}
