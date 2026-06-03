use quote::quote;
use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::ast::{
    ParseOption,
    attributes::{AttributeKey, AttributeValue, AttributeWriter, WriteAttribute},
    view::{ExprKind, ViewWriter, WriteView},
};

pub struct Attribute {
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: AttributeValue,
}

impl WriteView for Attribute {
    fn write(&self, writer: &mut ViewWriter) {
        match self.value {
            AttributeValue::LitStr(_) => {
                writer.write_str_unescaped(" ");
                self.key.write(writer);
                writer.write_str_unescaped("=\"");
                self.value.write(writer);
                writer.write_str_unescaped("\"");
            }
            AttributeValue::Expr(_) => {
                let key = &self.key;
                let value = &self.value;
                if matches!(key, AttributeKey::Expr(..)) {
                    writer.write_expr(ExprKind::Attribute, quote! { (#key, #value) });
                } else {
                    writer.write_expr(ExprKind::AttributeUnescaped, quote! { (#key, #value) });
                }
            }
        }
    }
}

impl WriteAttribute for Attribute {
    fn write(&self, writer: &mut AttributeWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.insert(quote! { #key }, quote! { #value });
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Attribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident::peek_any) || input.peek(Paren)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attribute {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
