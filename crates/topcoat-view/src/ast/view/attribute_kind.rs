use syn::{
    Token,
    parse::{Parse, ParseStream},
};

/// The prefix sigil that selects how an attribute is interpreted.
///
/// - [`Static`](Self::Static) — no prefix: a plain HTML attribute.
/// - [`Bind`](Self::Bind) — `:` prefix: a one-way binding from a reactive
///   expression to a DOM attribute or property.
/// - [`Event`](Self::Event) — `@` prefix: a DOM event handler.
pub enum AttributeKind {
    Static,
    Bind(Token![:]),
    Event(Token![@]),
}

impl AttributeKind {
    pub fn peek(input: ParseStream) -> bool {
        input.peek(Token![:]) || input.peek(Token![@])
    }
}

impl Parse for AttributeKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![:]) {
            Ok(Self::Bind(input.parse()?))
        } else if input.peek(Token![@]) {
            Ok(Self::Event(input.parse()?))
        } else {
            Ok(Self::Static)
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for AttributeKind {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Static => {}
            Self::Bind(token) => token.pretty_print(printer),
            Self::Event(token) => token.pretty_print(printer),
        }
    }
}
