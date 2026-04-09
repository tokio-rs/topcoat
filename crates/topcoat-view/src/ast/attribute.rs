use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, LitStr, Token, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::{ast::ParseOption, output::ViewWriter};

pub struct Attribute {
    pub name: Ident,
    pub eq: Token![=],
    pub value: AttributeValue,
}

impl Attribute {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let name = self.name.to_string();
        writer.push_str(&name);
        writer.push_str("=\"");
        self.value.write(writer);
        writer.push_str("\"");
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Attribute {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident) && input.peek2(Token![=])
    }
}

pub enum AttributeValue {
    Expr { paren: Paren, expr: Expr },
    LitStr(LitStr),
}

impl AttributeValue {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Expr { expr, .. } => writer.push_expr(expr.to_token_stream()),
            Self::LitStr(inner) => writer.push_escaped(&inner.value()),
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
        }
    }
}

pub struct Attributes {
    pub items: Vec<Attribute>,
}

impl Attributes {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        for item in &self.items {
            writer.push_str(" ");
            item.write(writer);
        }
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
