use quote::quote;
use syn::{
    Expr, ExprBreak, ExprContinue, Pat, Token,
    parse::{Parse, ParseStream},
};

use crate::ast::{
    ParseOption,
    attributes::{AttributeWriter, WriteAttribute},
    template::TemplateBlock,
    view::{ViewWriter, WriteView},
};

/// A `for pat in expr { ... }` loop in view-body position. The body is
/// rendered once per iteration.
pub struct TemplateForLoop<T> {
    pub for_token: Token![for],
    pub pat: Box<Pat>,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: TemplateBlock<T>,
}

impl<T: WriteView> WriteView for TemplateForLoop<T> {
    fn write(&self, writer: &mut ViewWriter) {
        writer.for_loop(&self.pat, &self.expr, |writer| {
            self.body.write(writer);
        });
    }
}

impl<T: WriteAttribute> WriteAttribute for TemplateForLoop<T> {
    fn write(&self, writer: &mut AttributeWriter) {
        writer.for_loop(&self.pat, &self.expr, |writer| {
            self.body.write(writer);
        });
    }
}

impl<T: Parse> Parse for TemplateForLoop<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            for_token: input.parse()?,
            pat: Box::new(input.call(Pat::parse_single)?),
            in_token: input.parse()?,
            expr: Box::new(input.call(Expr::parse_without_eager_brace)?),
            body: input.parse()?,
        })
    }
}

impl<T: Parse> ParseOption for TemplateForLoop<T> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![for])
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for TemplateForLoop<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.for_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.in_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

/// A `continue;` statement.
pub struct TemplateContinue {
    pub expr_continue: ExprContinue,
    pub semi_token: Token![;],
}

impl WriteView for TemplateContinue {
    fn write(&self, writer: &mut ViewWriter) {
        let expr_continue = &self.expr_continue;
        writer.statement(quote! { #expr_continue; });
    }
}

impl WriteAttribute for TemplateContinue {
    fn write(&self, writer: &mut AttributeWriter) {
        let expr_continue = &self.expr_continue;
        writer.statement(quote! { #expr_continue; });
    }
}

impl Parse for TemplateContinue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_continue: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateContinue {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![continue])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateContinue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use quote::ToTokens;

        self.expr_continue
            .to_token_stream()
            .to_string()
            .pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

/// A `break;` statement.
pub struct TemplateBreak {
    pub expr_break: ExprBreak,
    pub semi_token: Token![;],
}

impl WriteView for TemplateBreak {
    fn write(&self, writer: &mut ViewWriter) {
        let expr_break = &self.expr_break;
        writer.statement(quote! { #expr_break; });
    }
}

impl WriteAttribute for TemplateBreak {
    fn write(&self, writer: &mut AttributeWriter) {
        let expr_break = &self.expr_break;
        writer.statement(quote! { #expr_break; });
    }
}

impl Parse for TemplateBreak {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_break: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateBreak {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![break])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateBreak {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use quote::ToTokens;

        self.expr_break
            .to_token_stream()
            .to_string()
            .pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}
