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
        // `match=` is an attribute named `match`, not the start of a match.
        input.peek(Token![match]) && !input.peek2(Token![=])
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
                if input.is_empty() {
                    input.parse()?
                } else {
                    Some(input.parse()?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::view::Node;
    use quote::ToTokens;

    fn parse(source: &str) -> TemplateMatch<Node> {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_match_with_one_arm() {
        let m = parse(r#"match v { _ => "x", }"#);
        assert_eq!(m.expr.to_token_stream().to_string(), "v");
        assert_eq!(m.arms.len(), 1);
        assert!(m.arms[0].comma.is_some());
    }

    #[test]
    fn parses_match_with_multiple_arms() {
        let m = parse(
            r#"match status {
                A => "a",
                B => "b",
                C => "c",
            }"#,
        );
        assert_eq!(m.arms.len(), 3);
    }

    #[test]
    fn parses_arm_with_guard() {
        let m = parse(r#"match v { x if x > 0 => "pos", _ => "neg", }"#);
        let guard = m.arms[0].guard.as_ref().expect("guard");
        assert_eq!(guard.1.to_token_stream().to_string(), "x > 0");
    }

    #[test]
    fn parses_struct_pattern() {
        let m = parse(r#"match v { Foo { bar } => (bar), _ => "x", }"#);
        assert_eq!(m.arms[0].pat.to_token_stream().to_string(), "Foo { bar }");
    }

    #[test]
    fn allows_or_pattern_with_leading_vert() {
        // `parse_multi_with_leading_vert` accepts a leading `|`; it's not
        // preserved in the resulting pattern.
        let m = parse(r#"match v { | A | B => "ab", _ => "x", }"#);
        let rendered = m.arms[0].pat.to_token_stream().to_string();
        assert!(rendered.contains('A'));
        assert!(rendered.contains('B'));
        assert!(rendered.contains('|'));
    }

    #[test]
    fn final_arm_may_omit_trailing_comma() {
        let m = parse(r#"match v { _ => "x" }"#);
        assert!(m.arms[0].comma.is_none());
    }

    #[test]
    fn parses_block_body() {
        let m = parse(r#"match v { _ => { "a" "b" } }"#);
        let Node::Block(_) = &*m.arms[0].body else {
            panic!("expected block body");
        };
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
    fn match_equals_is_an_attribute_not_a_match() {
        assert!(peeks(TemplateMatch::<Node>::peek, "match v { _ => \"x\" }"));
        assert!(!peeks(TemplateMatch::<Node>::peek, r#"match="x""#));
    }
}
