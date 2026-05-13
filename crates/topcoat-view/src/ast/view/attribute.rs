use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, ExprCall, ExprMacro, ExprPath, Ident, LitBool, LitFloat, LitInt, LitStr, Macro,
    MacroDelimiter, Path, Token, braced, bracketed,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket, Paren},
};

use crate::ast::{ParseOption, view::ViewWriter};

/// A single `name=value` attribute on an [`Element`](super::Element) or
/// [`Component`](super::Component).
pub struct Attribute {
    pub name: Ident,
    pub eq: Token![=],
    pub value: AttributeValue,
}

impl Attribute {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let name = self.name.to_string();
        writer.write_str_unescaped(&name);
        writer.write_str_unescaped("=\"");
        self.value.write(writer);
        writer.write_str_unescaped("\"");
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            // Accept Rust keywords as attribute names.
            name: Ident::parse_any(input)?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Attribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident::peek_any) && input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attribute {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.name.pretty_print(printer);
        self.eq.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub enum AttributeValue {
    Expr { paren: Paren, expr: Box<Expr> },
    LitStr(LitStr),
    LitInt(LitInt),
    LitFloat(LitFloat),
    LitBool(LitBool),
    Path(Path),
    Call(ExprCall),
    Macro(ExprMacro),
}

impl AttributeValue {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Expr { expr, .. } => writer.write_expr(expr.to_token_stream()),
            Self::LitStr(inner) => writer.write_str(&inner.value()),
            Self::LitInt(inner) => writer.write_str(inner.base10_digits()),
            Self::LitFloat(inner) => writer.write_str(inner.base10_digits()),
            Self::LitBool(inner) => writer.write_str(if inner.value { "true" } else { "false" }),
            Self::Path(inner) => writer.write_expr(inner.to_token_stream()),
            Self::Call(inner) => writer.write_expr(inner.to_token_stream()),
            Self::Macro(inner) => writer.write_expr(inner.to_token_stream()),
        }
    }
}

impl Parse for AttributeValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Paren) {
            let content;
            Ok(Self::Expr {
                paren: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else if lookahead.peek(LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else if lookahead.peek(LitInt) {
            Ok(Self::LitInt(input.parse()?))
        } else if lookahead.peek(LitFloat) {
            Ok(Self::LitFloat(input.parse()?))
        } else if lookahead.peek(LitBool) {
            Ok(Self::LitBool(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) || lookahead.peek(Token![::]) {
            let path: Path = input.parse()?;
            if input.peek(Token![!]) {
                let bang_token = input.parse()?;
                let content;
                let delimiter = if input.peek(Paren) {
                    MacroDelimiter::Paren(parenthesized!(content in input))
                } else if input.peek(Bracket) {
                    MacroDelimiter::Bracket(bracketed!(content in input))
                } else if input.peek(Brace) {
                    MacroDelimiter::Brace(braced!(content in input))
                } else {
                    return Err(input.error("expected `(`, `[`, or `{` after `!`"));
                };
                Ok(Self::Macro(ExprMacro {
                    attrs: Vec::new(),
                    mac: Macro {
                        path,
                        bang_token,
                        delimiter,
                        tokens: content.parse()?,
                    },
                }))
            } else if input.peek(Paren) {
                let content;
                let paren_token = parenthesized!(content in input);
                let args = Punctuated::parse_terminated(&content)?;
                Ok(Self::Call(ExprCall {
                    attrs: Vec::new(),
                    func: Box::new(Expr::Path(ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path,
                    })),
                    paren_token,
                    args,
                }))
            } else {
                Ok(Self::Path(path))
            }
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr { expr, .. } => quote! {{ #expr }}.to_tokens(tokens),
            Self::LitStr(inner) => inner.to_tokens(tokens),
            Self::LitInt(inner) => inner.to_tokens(tokens),
            Self::LitFloat(inner) => inner.to_tokens(tokens),
            Self::LitBool(inner) => inner.to_tokens(tokens),
            Self::Path(inner) => inner.to_tokens(tokens),
            Self::Call(inner) => inner.to_tokens(tokens),
            Self::Macro(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for AttributeValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::LitStr(inner) => inner.pretty_print(printer),
            Self::LitInt(inner) => syn::Lit::Int(inner.clone()).pretty_print(printer),
            Self::LitFloat(inner) => syn::Lit::Float(inner.clone()).pretty_print(printer),
            Self::LitBool(inner) => syn::Lit::Bool(inner.clone()).pretty_print(printer),
            Self::Path(inner) => inner.pretty_print(printer),
            Self::Expr { paren, expr } => {
                use topcoat_pretty::{BreakMode, Delim};
                paren.pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                    expr.pretty_print(printer);
                });
            }
            Self::Call(inner) => {
                syn::Expr::Call(inner.clone()).pretty_print(printer);
            }
            Self::Macro(inner) => {
                syn::Expr::Macro(inner.clone()).pretty_print(printer);
            }
        }
    }
}

/// The full list of attributes attached to a single tag.
pub struct Attributes {
    pub items: Vec<Attribute>,
}

impl Attributes {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        for item in &self.items {
            writer.write_str_unescaped(" ");
            item.write(writer);
        }
    }

    /// Returns `true` if `self` has no attributes.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while let Some(attribute) = input.call(Attribute::parse_option)? {
            items.push(attribute);
        }
        Ok(Self { items })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Attributes {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        if self.items.is_empty() {
            return;
        }
        for item in &self.items {
            printer.scan_break();
            " ".pretty_print(printer);
            item.pretty_print(printer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_string_valued_attribute() {
        let attr: Attribute = syn::parse_str(r#"href="/about""#).unwrap();
        assert_eq!(attr.name.to_string(), "href");
        let AttributeValue::LitStr(lit) = &attr.value else {
            panic!("expected literal value");
        };
        assert_eq!(lit.value(), "/about");
    }

    #[test]
    fn parses_expression_valued_attribute() {
        let attr: Attribute = syn::parse_str("href=(url)").unwrap();
        assert!(matches!(attr.value, AttributeValue::Expr { .. }));
    }

    #[test]
    fn parses_multiple_attributes() {
        let attrs: Attributes = syn::parse_str(r#"type="text" name="q""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert_eq!(attrs.items[0].name.to_string(), "type");
        assert_eq!(attrs.items[1].name.to_string(), "name");
    }

    #[test]
    fn empty_input_yields_empty_attributes() {
        let attrs: Attributes = syn::parse_str("").unwrap();
        assert!(attrs.is_empty());
    }

    #[test]
    fn parses_call_valued_attribute() {
        let attr: Attribute = syn::parse_str("onclick=handle()").unwrap();
        assert_eq!(attr.name.to_string(), "onclick");
        assert!(matches!(attr.value, AttributeValue::Call(_)));
    }

    #[test]
    fn parses_call_valued_attribute_with_path() {
        let attr: Attribute = syn::parse_str("onclick=handlers::click(state)").unwrap();
        assert!(matches!(attr.value, AttributeValue::Call(_)));
    }

    #[test]
    fn parses_macro_valued_attribute() {
        let attr: Attribute = syn::parse_str(r#"title=tr!("hello")"#).unwrap();
        assert_eq!(attr.name.to_string(), "title");
        assert!(matches!(attr.value, AttributeValue::Macro(_)));
    }

    #[test]
    fn parses_call_followed_by_attribute() {
        let attrs: Attributes = syn::parse_str(r#"onclick=handle() class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert_eq!(attrs.items[0].name.to_string(), "onclick");
        assert!(matches!(attrs.items[0].value, AttributeValue::Call(_)));
        assert_eq!(attrs.items[1].name.to_string(), "class");
        assert!(matches!(attrs.items[1].value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn parses_macro_followed_by_attribute() {
        let attrs: Attributes = syn::parse_str(r#"title=tr!("hello") class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert!(matches!(attrs.items[0].value, AttributeValue::Macro(_)));
        assert!(matches!(attrs.items[1].value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn parses_call_between_attributes() {
        let attrs: Attributes = syn::parse_str(r#"id="x" onclick=handle() class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 3);
        assert!(matches!(attrs.items[1].value, AttributeValue::Call(_)));
    }

    #[test]
    fn parses_int_valued_attribute() {
        let attr: Attribute = syn::parse_str("tabindex=0").unwrap();
        let AttributeValue::LitInt(lit) = &attr.value else {
            panic!("expected int literal");
        };
        assert_eq!(lit.base10_digits(), "0");
    }

    #[test]
    fn parses_float_valued_attribute() {
        let attr: Attribute = syn::parse_str("opacity=0.5").unwrap();
        let AttributeValue::LitFloat(lit) = &attr.value else {
            panic!("expected float literal");
        };
        assert_eq!(lit.base10_digits(), "0.5");
    }

    #[test]
    fn parses_true_valued_attribute() {
        let attr: Attribute = syn::parse_str("disabled=true").unwrap();
        let AttributeValue::LitBool(lit) = &attr.value else {
            panic!("expected bool literal");
        };
        assert!(lit.value);
    }

    #[test]
    fn parses_false_valued_attribute() {
        let attr: Attribute = syn::parse_str("disabled=false").unwrap();
        let AttributeValue::LitBool(lit) = &attr.value else {
            panic!("expected bool literal");
        };
        assert!(!lit.value);
    }

    #[test]
    fn parses_path_valued_attribute() {
        let attr: Attribute = syn::parse_str("href=url").unwrap();
        let AttributeValue::Path(path) = &attr.value else {
            panic!("expected path");
        };
        assert!(path.is_ident("url"));
    }

    #[test]
    fn parses_multi_segment_path_valued_attribute() {
        let attr: Attribute = syn::parse_str("href=routes::home").unwrap();
        let AttributeValue::Path(path) = &attr.value else {
            panic!("expected path");
        };
        assert_eq!(path.segments.len(), 2);
    }

    #[test]
    fn parses_int_followed_by_attribute() {
        let attrs: Attributes = syn::parse_str(r#"tabindex=1 class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert!(matches!(attrs.items[0].value, AttributeValue::LitInt(_)));
        assert!(matches!(attrs.items[1].value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn parses_bool_followed_by_attribute() {
        let attrs: Attributes = syn::parse_str(r#"disabled=true class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert!(matches!(attrs.items[0].value, AttributeValue::LitBool(_)));
        assert!(matches!(attrs.items[1].value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn parses_path_followed_by_attribute() {
        let attrs: Attributes = syn::parse_str(r#"href=url class="foo""#).unwrap();
        assert_eq!(attrs.items.len(), 2);
        assert!(matches!(attrs.items[0].value, AttributeValue::Path(_)));
        assert!(matches!(attrs.items[1].value, AttributeValue::LitStr(_)));
    }
}
