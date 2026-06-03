use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use crate::ast::{
    ParseOption,
    attributes::{AttributeWriter, WriteAttribute},
    template::TemplateBlock,
    view::{ViewWriter, WriteView},
};

/// An `if cond { ... } else { ... }` chain in view-body position.
pub struct TemplateIf<T> {
    pub if_token: Token![if],
    pub cond: syn::Expr,
    pub then_branch: TemplateBlock<T>,
    pub else_branch: Option<TemplateElse<T>>,
}

impl<T: WriteView> WriteView for TemplateIf<T> {
    fn write(&self, writer: &mut ViewWriter) {
        writer.if_else(&self.cond, |then_writer, else_writer| {
            self.then_branch.write(then_writer);
            if let Some(else_branch) = self.else_branch.as_ref() {
                else_branch.write(else_writer);
            }
        });
    }
}

impl<T: WriteAttribute> WriteAttribute for TemplateIf<T> {
    fn write(&self, writer: &mut AttributeWriter) {
        writer.if_else(&self.cond, |then_writer, else_writer| {
            self.then_branch.write(then_writer);
            if let Some(else_branch) = self.else_branch.as_ref() {
                else_branch.write(else_writer);
            }
        });
    }
}

impl<T: Parse> Parse for TemplateIf<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            if_token: input.parse()?,
            cond: input.call(syn::Expr::parse_without_eager_brace)?,
            then_branch: input.parse()?,
            else_branch: input.call(TemplateElse::parse_option)?,
        })
    }
}

impl<T: Parse> ParseOption for TemplateIf<T> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![if])
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for TemplateIf<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.if_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.cond.pretty_print(printer);
        " ".pretty_print(printer);
        self.then_branch.pretty_print(printer);
        self.else_branch.pretty_print(printer);
    }
}

/// The trailing `else if ...` or `else { ... }` of a [`TemplateIf`].
pub enum TemplateElse<T> {
    ElseIf {
        else_token: Token![else],
        template_if: Box<TemplateIf<T>>,
    },
    Else {
        else_token: Token![else],
        then_branch: TemplateBlock<T>,
    },
}

impl<T: WriteView> WriteView for TemplateElse<T> {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::ElseIf { template_if, .. } => template_if.write(writer),
            Self::Else { then_branch, .. } => then_branch.write(writer),
        }
    }
}

impl<T: WriteAttribute> WriteAttribute for TemplateElse<T> {
    fn write(&self, writer: &mut AttributeWriter) {
        match self {
            Self::ElseIf { template_if, .. } => template_if.write(writer),
            Self::Else { then_branch, .. } => then_branch.write(writer),
        }
    }
}

impl<T: Parse> Parse for TemplateElse<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let else_token: Token![else] = input.parse()?;
        if input.peek(Token![if]) {
            Ok(Self::ElseIf {
                else_token,
                template_if: input.parse()?,
            })
        } else {
            Ok(Self::Else {
                else_token,
                then_branch: input.parse()?,
            })
        }
    }
}

impl<T: Parse> ParseOption for TemplateElse<T> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![else])
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for TemplateElse<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::ElseIf {
                else_token,
                template_if,
            } => {
                " ".pretty_print(printer);
                else_token.pretty_print(printer);
                " ".pretty_print(printer);
                template_if.pretty_print(printer);
            }
            Self::Else {
                else_token,
                then_branch,
            } => {
                " ".pretty_print(printer);
                else_token.pretty_print(printer);
                " ".pretty_print(printer);
                then_branch.pretty_print(printer);
            }
        }
    }
}
