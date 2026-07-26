use syn::{
    Expr, Ident, Token, braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket},
};

use topcoat_view_grammar::view::View;

/// The value of a [`MailField`](super::MailField), shaped by the field's
/// name.
pub enum FieldValue {
    /// A braced `view!` body: the value of the `html` field.
    Html(HtmlValue),
    /// A bracketed value list: the values of an additive field.
    List(ListValue),
    /// A single expression: any other value.
    Expr(Box<Expr>),
}

impl FieldValue {
    /// Parses the value shape the field named `name` expects: a braced
    /// `view!` body for `html`, a bracketed list for an additive field, and
    /// a single expression otherwise, including for an additive field given
    /// one value.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid syntax for that shape.
    pub fn parse_named(name: &Ident, input: ParseStream) -> syn::Result<Self> {
        if name == "html" && input.peek(Brace) {
            Ok(Self::Html(input.parse()?))
        } else if is_additive(name) && input.peek(Bracket) {
            Ok(Self::List(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

/// A braced `view!` body, including the optional leading `cx =>` argument
/// that names the request context dynamic parts render against.
pub struct HtmlValue {
    pub brace_token: Brace,
    pub view: View,
}

impl Parse for HtmlValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace_token: braced!(content in input),
            view: content.parse()?,
        })
    }
}

/// A bracketed value list, like `[a, b, c]`.
pub struct ListValue {
    pub bracket_token: Bracket,
    pub values: Punctuated<Expr, Token![,]>,
}

impl Parse for ListValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: bracketed!(content in input),
            values: Punctuated::parse_terminated(&content)?,
        })
    }
}

/// Whether the field named `name` appends through the builder rather than
/// setting a single value, and so accepts a bracketed value list.
fn is_additive(name: &Ident) -> bool {
    ["to", "cc", "bcc", "reply_to", "attachments", "headers"]
        .iter()
        .any(|additive| name == additive)
}
