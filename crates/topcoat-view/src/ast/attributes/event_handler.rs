use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{AttributeKey, AttributeWriter, WriteAttribute},
    template::TemplateOrRuntimeExpr,
    view::{ExprKind, ViewWriter, WriteView},
};

/// The value attached to an [`EventHandler`]. Either a Rust expression
/// (`(expr)` or `$(expr)`) compiled to a handler, or a string literal holding
/// raw JavaScript that runs on the event.
pub enum EventHandlerValue {
    Expr(Box<TemplateOrRuntimeExpr>),
    LitStr(LitStr),
}

impl Parse for EventHandlerValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if TemplateOrRuntimeExpr::peek(input) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for EventHandlerValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(inner) => inner.to_tokens(tokens),
            Self::LitStr(inner) => quote! {
                ::topcoat::runtime::Expr::new(
                    |_: ::topcoat::runtime::Event| {},
                    ::topcoat::view::ViewPart::from(#inner),
                )
            }
            .to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for EventHandlerValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Expr(inner) => inner.pretty_print(printer),
            Self::LitStr(inner) => inner.pretty_print(printer),
        }
    }
}

/// An `@name=(expr)`, `@name=$(expr)`, or `@name="js"` attribute: a DOM event handler.
pub struct EventHandler {
    pub at: Token![@],
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: EventHandlerValue,
}

impl WriteView for EventHandler {
    fn write(&self, writer: &mut ViewWriter) {
        let key = &self.key;
        let value = &self.value;
        match value {
            EventHandlerValue::LitStr(value) => {
                writer.write_str_unescaped("data-topcoat-on:");
                key.write(writer);
                writer.write_str_unescaped("=\"");
                writer.write_str(&value.value());
                writer.write_str_unescaped("\"");
            }
            EventHandlerValue::Expr(value) => {
                writer.write_expr(
                    ExprKind::Attributes,
                    quote! {
                        ::topcoat::runtime::EventHandler::new(
                            #key,
                            #value,
                        )
                    },
                );
            }
        }
    }
}

impl WriteAttribute for EventHandler {
    fn write(&self, writer: &mut AttributeWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.insert_block(
            1,
            quote! {
                {
                    let __key = ::core::convert::Into::<::std::string::String>::into(#key);
                    let (_, __js) = #value.into_evaluated_and_js();
                    __attrs.insert(::std::format!("data-topcoat-on:{}", __key), __js);
                }
            },
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> EventHandler {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<EventHandler>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_template_expr_value() {
        let handler = parse("@click=(handler)");
        assert!(matches!(handler.key, AttributeKey::Ident(_)));
        let EventHandlerValue::Expr(expr) = &handler.value else {
            panic!("expected expression value");
        };
        assert!(matches!(**expr, TemplateOrRuntimeExpr::Template(_)));
    }

    #[test]
    fn parses_runtime_expr_value() {
        let handler = parse("@click=$(handler)");
        let EventHandlerValue::Expr(expr) = &handler.value else {
            panic!("expected expression value");
        };
        assert!(matches!(**expr, TemplateOrRuntimeExpr::Runtime(_)));
    }

    #[test]
    fn parses_literal_js_value() {
        let handler = parse(r#"@click="alert(1)""#);
        let EventHandlerValue::LitStr(lit) = &handler.value else {
            panic!("expected literal value");
        };
        assert_eq!(lit.value(), "alert(1)");
    }

    #[test]
    fn parses_expression_key() {
        let handler = parse("@(name)=(handler)");
        assert!(matches!(handler.key, AttributeKey::Expr(_)));
    }

    #[test]
    fn rejects_missing_value() {
        assert!(parse_err("@click").contains("expected `=`"));
    }
}
