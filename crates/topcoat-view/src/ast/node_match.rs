use std::ops::Deref;

use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream},
    token::Brace,
};

use crate::{
    ast::{node::Node, parse_option::ParseOption},
    output::{ViewWriter, ViewWriterMatch},
};

/// A `match expr { ... }` expression in view-body position.
pub struct NodeMatch {
    pub match_token: Token![match],
    pub expr: Box<Expr>,
    pub brace_token: Brace,
    pub arms: Vec<NodeMatchArm>,
}

impl NodeMatch {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let mut writer = writer.begin_match(&self.expr);
        for arm in &self.arms {
            arm.write(&mut writer);
        }
    }
}

impl Parse for NodeMatch {
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

impl ParseOption for NodeMatch {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![match])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeMatch {
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

/// A single arm of a [`NodeMatch`]: `pat (if guard)? => body`.
pub struct NodeMatchArm {
    pub pat: Pat,
    pub guard: Option<(Token![if], Box<Expr>)>,
    pub fat_arrow_token: Token![=>],
    pub body: Box<Node>,
    pub comma: Option<Token![,]>,
}

impl NodeMatchArm {
    pub(crate) fn write<'a>(&'a self, writer: &mut ViewWriterMatch<'a>) {
        let mut writer = writer.begin_arm(
            &self.pat,
            self.guard.as_ref().map(|(_, guard)| guard.deref()),
        );
        self.body.write(&mut writer);
    }
}

impl Parse for NodeMatchArm {
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
impl topcoat_pretty::PrettyPrint for NodeMatchArm {
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
