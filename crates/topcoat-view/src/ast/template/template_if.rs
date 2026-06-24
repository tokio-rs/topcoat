use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
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
        // `if=` is an attribute named `if`, not the start of a conditional.
        input.peek(Token![if]) && !input.peek2(Token![=])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::view::Nodes;
    use quote::ToTokens;

    fn parse(source: &str) -> TemplateIf<Nodes> {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_if_without_else() {
        let if_ = parse(r#"if cond { "yes" }"#);
        assert_eq!(if_.cond.to_token_stream().to_string(), "cond");
        assert_eq!(if_.then_branch.children.len(), 1);
        assert!(if_.else_branch.is_none());
    }

    #[test]
    fn parses_if_else() {
        let if_ = parse(r#"if cond { "yes" } else { "no" }"#);
        assert!(matches!(if_.else_branch, Some(TemplateElse::Else { .. })));
    }

    #[test]
    fn parses_else_if_chain() {
        let if_ = parse(r#"if a { "a" } else if b { "b" } else { "c" }"#);
        let Some(TemplateElse::ElseIf { template_if, .. }) = if_.else_branch else {
            panic!("expected else-if chain");
        };
        assert!(matches!(
            template_if.else_branch,
            Some(TemplateElse::Else { .. }),
        ));
    }

    #[test]
    fn parses_complex_condition() {
        // `parse_without_eager_brace` lets the `{` start the body rather than
        // being eaten by a struct-literal in the condition.
        let if_ = parse(r#"if user.is_some() && active { "x" }"#);
        assert_eq!(
            if_.cond.to_token_stream().to_string(),
            "user . is_some () && active",
        );
    }

    #[test]
    fn parses_empty_branches() {
        let if_ = parse(r"if c {} else {}");
        assert!(if_.then_branch.children.is_empty());
        let Some(TemplateElse::Else { then_branch, .. }) = if_.else_branch else {
            panic!("expected else branch");
        };
        assert!(then_branch.children.is_empty());
    }

    /// Evaluates a `peek` against `source`, draining the remaining tokens so the
    /// surrounding `parse_str` doesn't error on the unconsumed input.
    fn peeks(peek: fn(ParseStream) -> bool, source: &str) -> bool {
        let parser = move |input: ParseStream| -> syn::Result<bool> {
            let peeked = peek(input);
            input.parse::<proc_macro2::TokenStream>()?;
            Ok(peeked)
        };
        syn::parse::Parser::parse_str(parser, source).unwrap()
    }

    #[test]
    fn if_equals_is_an_attribute_not_a_conditional() {
        assert!(peeks(TemplateIf::<Nodes>::peek, "if cond {}"));
        assert!(!peeks(TemplateIf::<Nodes>::peek, r#"if="x""#));
    }
}
