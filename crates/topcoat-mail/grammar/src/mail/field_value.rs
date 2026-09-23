use syn::{
    Expr, Ident,
    parse::{Parse, ParseStream},
    token::Brace,
};
use topcoat_view_grammar::view::View;

/// The value of a [`MailField`](super::MailField). Its form depends on the
/// field name.
pub enum FieldValue {
    /// A braced `view!` body: the value of the `html` field.
    Html(HtmlValue),
    /// An expression: the value of every other field, and of an `html`
    /// field without braces.
    Expr(Box<Expr>),
}

impl FieldValue {
    /// Parses the value of the field named `name`: a braced `view!` body for
    /// `html`, and an expression otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid syntax for that shape.
    pub fn parse_named(name: &Ident, input: ParseStream) -> syn::Result<Self> {
        if name == "html" && input.peek(Brace) {
            Ok(Self::Html(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

/// A braced `view!` body, including the optional leading `cx =>` that names
/// the request context.
pub struct HtmlValue {
    /// The braces around the body.
    pub brace_token: Brace,
    /// The `view!` body inside the braces.
    pub view: View,
}

impl Parse for HtmlValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace_token: syn::braced!(content in input),
            view: content.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FieldValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Html(html) => html.pretty_print(printer),
            Self::Expr(expr) => expr.pretty_print(printer),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HtmlValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::{BreakMode, Delim};

        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.view.pretty_print(printer);
            });
    }
}
