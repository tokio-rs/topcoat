use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream},
    token::Brace,
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{AttributeWriter, MatchArmsBuilder as AttributeMatchArmsBuilder, WriteAttribute},
    view::{MatchArmsBuilder, ViewWriter, WriteView},
};

/// A `match expr { ... }` expression in view-body position.
pub struct TemplateMatch<B> {
    pub match_token: Token![match],
    pub expr: Box<Expr>,
    pub brace_token: Brace,
    pub arms: Vec<TemplateMatchArm<B>>,
}

impl<B: WriteView> WriteView for TemplateMatch<B> {
    fn write(&self, writer: &mut ViewWriter) {
        writer.match_expr(&self.expr, |arms| {
            for arm in &self.arms {
                arm.write(arms);
            }
        });
    }
}

impl<B: WriteAttribute> WriteAttribute for TemplateMatch<B> {
    fn write(&self, writer: &mut AttributeWriter) {
        writer.match_expr(&self.expr, |arms| {
            for arm in &self.arms {
                arm.write_attribute(arms);
            }
        });
    }
}

impl<B: Parse> Parse for TemplateMatch<B> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            match_token: input.parse()?,
            expr: Box::new(input.call(Expr::parse_without_eager_brace)?),
            brace_token: syn::braced!(content in input),
            arms: {
                let mut arms = Vec::new();
                while !content.is_empty() {
                    arms.push(content.parse()?);
                }
                arms
            },
        })
    }
}

impl<B: Parse> ParseOption for TemplateMatch<B> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![match])
    }
}

#[cfg(feature = "pretty")]
impl<B> topcoat_pretty::PrettyPrint for TemplateMatch<B>
where
    TemplateMatchArm<B>: topcoat_pretty::PrettyPrint,
{
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use topcoat_pretty::{BreakMode, Delim};

        self.match_token.pretty_print(printer);

        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);

        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                for (index, arm) in self.arms.iter().enumerate() {
                    arm.pretty_print(printer);
                    if index < self.arms.len() - 1 {
                        printer.scan_force_break();
                        printer.scan_break();
                    }
                }
            });
    }
}

/// A single arm of a [`TemplateMatch`]: `pat (if guard)? => body`.
pub struct TemplateMatchArm<B> {
    pub pat: Pat,
    pub guard: Option<(Token![if], Box<Expr>)>,
    pub fat_arrow_token: Token![=>],
    pub body: Box<B>,
    pub comma: Option<Token![,]>,
}

#[allow(private_bounds)]
impl<B: WriteView> TemplateMatchArm<B> {
    pub(crate) fn write(&self, arms: &mut MatchArmsBuilder) {
        arms.arm(
            &self.pat,
            self.guard.as_ref().map(|(_, expr)| expr.as_ref()),
            |writer| self.body.write(writer),
        );
    }
}

#[allow(private_bounds)]
impl<B: WriteAttribute> TemplateMatchArm<B> {
    pub(crate) fn write_attribute(&self, arms: &mut AttributeMatchArmsBuilder) {
        arms.arm(
            &self.pat,
            self.guard.as_ref().map(|(_, expr)| expr.as_ref()),
            |writer| self.body.write(writer),
        );
    }
}

impl<B: Parse> Parse for TemplateMatchArm<B> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            pat: Pat::parse_multi_with_leading_vert(input)?,
            guard: {
                if input.peek(Token![if]) {
                    let if_token: Token![if] = input.parse()?;
                    let guard: Expr = input.parse()?;
                    Some((if_token, Box::new(guard)))
                } else {
                    None
                }
            },
            fat_arrow_token: input.parse()?,
            body: Box::new(input.parse()?),
            comma: {
                if !input.is_empty() {
                    Some(input.parse()?)
                } else {
                    input.parse()?
                }
            },
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateMatchArm<crate::ast::view::Node> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.fat_arrow_token.pretty_print(printer);
        if let Some((if_token, expr)) = &self.guard {
            " ".pretty_print(printer);
            if_token.pretty_print(printer);
            " ".pretty_print(printer);
            expr.pretty_print(printer);
        }
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
        if !self.body.is_block() {
            if let Some(comma) = &self.comma {
                comma.pretty_print(printer);
            } else {
                ",".pretty_print(printer);
            }
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateMatchArm<crate::ast::attributes::AttributeNode> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.fat_arrow_token.pretty_print(printer);
        if let Some((if_token, expr)) = &self.guard {
            " ".pretty_print(printer);
            if_token.pretty_print(printer);
            " ".pretty_print(printer);
            expr.pretty_print(printer);
        }
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
        if let Some(comma) = &self.comma {
            comma.pretty_print(printer);
        } else {
            ",".pretty_print(printer);
        }
    }
}
