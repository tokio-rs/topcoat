use quote::quote;
use syn::{
    Ident, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::ast::{
    ParseOption,
    view::{AttributeKey, AttributeValue, TemplateExpr, ViewWriter, WriteView},
};

/// A plain `name=value` attribute on an [`Element`](super::Element) or
/// [`Component`](super::Component).
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
                writer.write_expr(quote! {
                    ::topcoat::view::Attribute::new(
                        #key,
                        #value,
                    )
                });
            }
        }
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

/// A `:name=(expr)` attribute — a one-way binding from a reactive expression to
/// a DOM attribute or property.
pub struct BindAttribute {
    pub colon: Token![:],
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: TemplateExpr,
}

impl WriteView for BindAttribute {
    fn write(&self, writer: &mut ViewWriter) {
        let key = &self.key;
        let expr = &self.value.expr;
        writer.write_expr(quote! {
            ::topcoat::runtime::BindAttribute::new(
                #key,
                ::topcoat::runtime::expr! { #expr },
            )
        });
    }
}

impl Parse for BindAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            colon: input.parse()?,
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for BindAttribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![:])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for BindAttribute {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.colon.pretty_print(printer);
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

/// An `@name=(expr)` attribute — a DOM event handler.
pub struct EventHandler {
    pub at: Token![@],
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: TemplateExpr,
}

impl WriteView for EventHandler {
    fn write(&self, writer: &mut ViewWriter) {
        let key = &self.key;
        let expr = &self.value.expr;
        writer.write_expr(quote! {
            ::topcoat::runtime::EventHandler::new(
                #key,
                ::topcoat::runtime::expr! { #expr },
            )
        });
    }
}

impl Parse for EventHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            at: input.parse()?,
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for EventHandler {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![@])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for EventHandler {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.at.pretty_print(printer);
        self.key.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
