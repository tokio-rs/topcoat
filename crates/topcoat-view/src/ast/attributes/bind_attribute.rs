use quote::quote;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{AttributeKey, AttributeWriter, WriteAttribute},
    template::TemplateOrRuntimeExpr,
    view::{ExprKind, ViewWriter, WriteView},
};

/// A `:name=(expr)` or `:name=$(expr)` attribute — a one-way binding to a DOM
/// attribute or property.
pub struct BindAttribute {
    pub colon: Token![:],
    pub key: AttributeKey,
    pub eq: Token![=],
    pub value: TemplateOrRuntimeExpr,
}

impl WriteView for BindAttribute {
    fn write(&self, writer: &mut ViewWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.write_expr(
            ExprKind::Attributes,
            quote! {
                ::topcoat::runtime::BindAttribute::new(#key, #value)
            },
        );
    }
}

impl WriteAttribute for BindAttribute {
    fn write(&self, writer: &mut AttributeWriter) {
        let key = &self.key;
        let value = &self.value;
        writer.insert_block(
            2,
            quote! {
                {
                    let __key = ::core::convert::Into::<::std::string::String>::into(#key);
                    let (__evaluated, __js) = #value.into_evaluated_and_js();
                    __attrs.insert(__key.clone(), __evaluated);
                    __attrs.insert(::std::format!("data-topcoat-bind:{}", __key), __js);
                }
            },
        );
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
