use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::ast::{
    ParseOption,
    view::{AttributeKey, AttributeKind, AttributeValue, ViewWriter, WriteView},
};

/// A single `name=value` attribute on an [`Element`](super::Element) or
/// [`Component`](super::Component).
pub struct Attribute {
    pub kind: AttributeKind,
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: AttributeValue,
}

impl WriteView for Attribute {
    fn write(&self, writer: &mut ViewWriter) {
        match self.kind {
            AttributeKind::Static => {
                writer.write_str_unescaped(" ");
                self.key.write(writer);
                writer.write_str_unescaped("=\"");
                self.value.write(writer);
                writer.write_str_unescaped("\"");
            }
            AttributeKind::Bind(_) => todo!(),
            AttributeKind::Event(_) => todo!(),
        }
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            kind: input.parse()?,
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Attribute {
    fn peek(input: ParseStream) -> bool {
        AttributeKind::peek(input) || input.peek(Ident::peek_any) || input.peek(Paren)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attribute {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.kind.pretty_print(printer);
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
