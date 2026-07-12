use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Lit, LitStr, Token};

use topcoat_core_grammar::ParseOption;

/// The parsed body of a `class!` invocation. Lowers to a
/// [`runtime::Class`](topcoat_view::Class).
///
/// Unlike `view!` and `attributes!`, the body takes no leading `cx =>`
/// argument: constructing a class list does not touch the request context.
/// The entries receive it later, when the surrounding attribute machinery
/// converts the class list into view parts.
pub struct Class {
    pub segments: Punctuated<ClassSegment, Token![,]>,
}

impl Parse for Class {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            segments: Punctuated::parse_terminated(input)?,
        })
    }
}

impl ToTokens for Class {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let segments: Vec<&ClassSegment> = self.segments.iter().collect();
        let mut entries: Vec<TokenStream> = Vec::new();

        // Runs of consecutive unconditional string literals collapse into a
        // single borrowed literal with the separating spaces baked in.
        let mut index = 0;
        while index < segments.len() {
            if let Some(first) = segments[index].as_unconditional_lit() {
                let mut values = Vec::new();
                while let Some(lit) = segments.get(index).and_then(|s| s.as_unconditional_lit()) {
                    let value = lit.value();
                    if !value.is_empty() {
                        values.push(value);
                    }
                    index += 1;
                }
                if !values.is_empty() {
                    let merged = LitStr::new(&values.join(" "), first.span());
                    entries.push(quote! { ::std::borrow::Cow::Borrowed(#merged) });
                }
            } else {
                entries.push(segments[index].entry_tokens());
                index += 1;
            }
        }

        let entries = entries_tuple(entries);
        quote! { ::topcoat::view::Class(#entries) }.to_tokens(tokens);
    }
}

impl topcoat_pretty::PrettyPrint for Class {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        for (index, segment) in self.segments.iter().enumerate() {
            segment.pretty_print(printer);
            if index < self.segments.len() - 1 {
                ",".pretty_print(printer);
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            } else {
                // A trailing comma is rendered only when the list breaks across
                // multiple lines, and vanishes when it stays on one line.
                printer.scan_text(",".into(), topcoat_pretty::TextMode::Break);
                printer.advance_cursor(",");
            }
        }
    }
}

/// Builds the `ClassEntries` value for the lowered entries.
///
/// A single entry is passed through bare; multiple entries form a tuple.
/// The runtime implements tuples up to twelve elements, so longer lists nest
/// chunks of twelve, which flatten again when the entries are written.
fn entries_tuple(mut entries: Vec<TokenStream>) -> TokenStream {
    loop {
        match entries.len() {
            0 => return quote! { () },
            1 => return entries.pop().unwrap(),
            2..=12 => return quote! { (#(#entries,)*) },
            _ => {
                entries = entries
                    .chunks(12)
                    .map(|chunk| quote! { (#(#chunk,)*) })
                    .collect();
            }
        }
    }
}

/// A single class list entry: an expression with an optional trailing
/// `if cond` or `if cond else alt` condition.
pub struct ClassSegment {
    pub value: Expr,
    pub condition: Option<ClassCondition>,
}

impl ClassSegment {
    /// Returns the segment's string literal if it is an unconditional
    /// literal, the only shape that can merge with its neighbors.
    fn as_unconditional_lit(&self) -> Option<&LitStr> {
        if self.condition.is_some() {
            return None;
        }
        as_lit_str(&self.value)
    }

    /// Lowers this segment to an expression implementing `ClassViewParts`.
    fn entry_tokens(&self) -> TokenStream {
        let value = value_tokens(&self.value);
        let Some(condition) = &self.condition else {
            return value;
        };
        let cond = &condition.condition;
        let Some(else_branch) = &condition.else_branch else {
            return quote! {
                if #cond {
                    ::core::option::Option::Some(#value)
                } else {
                    ::core::option::Option::None
                }
            };
        };
        let alt = value_tokens(&else_branch.value);
        // Two literal branches share the borrowed string type, so they lower
        // to a plain conditional. Anything else needs the branch enum to
        // unify the two types.
        if as_lit_str(&self.value).is_some() && as_lit_str(&else_branch.value).is_some() {
            quote! { if #cond { #value } else { #alt } }
        } else {
            quote! {
                if #cond {
                    ::topcoat::view::ClassBranch::Then(#value)
                } else {
                    ::topcoat::view::ClassBranch::Else(#alt)
                }
            }
        }
    }
}

impl Parse for ClassSegment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            value: input.parse()?,
            condition: input.call(ClassCondition::parse_option)?,
        })
    }
}

impl topcoat_pretty::PrettyPrint for ClassSegment {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.value.pretty_print(printer);
        if let Some(condition) = &self.condition {
            " if ".pretty_print(printer);
            condition.condition.pretty_print(printer);
            if let Some(else_branch) = &condition.else_branch {
                " else ".pretty_print(printer);
                else_branch.value.pretty_print(printer);
            }
        }
    }
}

/// Lowers a segment value to an entry expression, borrowing string literals
/// so they never allocate.
fn value_tokens(expr: &Expr) -> TokenStream {
    match as_lit_str(expr) {
        Some(lit) => quote! { ::std::borrow::Cow::Borrowed(#lit) },
        None => expr.to_token_stream(),
    }
}

/// Returns the string literal behind `expr`, if it is one.
fn as_lit_str(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit),
            attrs,
        }) if attrs.is_empty() => Some(lit),
        _ => None,
    }
}

/// The trailing `if cond` of a [`ClassSegment`], with an optional
/// `else alt`.
pub struct ClassCondition {
    pub if_token: Token![if],
    pub condition: Expr,
    pub else_branch: Option<ClassElse>,
}

impl Parse for ClassCondition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            if_token: input.parse()?,
            condition: input.parse()?,
            else_branch: input.call(ClassElse::parse_option)?,
        })
    }
}

impl ParseOption for ClassCondition {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![if])
    }
}

/// The `else alt` branch of a [`ClassCondition`].
pub struct ClassElse {
    pub else_token: Token![else],
    pub value: Expr,
}

impl Parse for ClassElse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            else_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for ClassElse {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![else])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Class {
        syn::parse_str(source).unwrap()
    }

    fn lower(source: &str) -> String {
        parse(source).to_token_stream().to_string()
    }

    #[test]
    fn empty_input_yields_no_segments() {
        assert!(parse("").segments.is_empty());
        assert!(lower("").contains("Class (())"));
    }

    #[test]
    fn collects_segments_in_order() {
        let class = parse(r#""btn", extra, "active" if is_active"#);
        assert_eq!(class.segments.len(), 3);
        assert!(class.segments[0].condition.is_none());
        assert!(class.segments[2].condition.is_some());
    }

    #[test]
    fn allows_a_trailing_comma() {
        assert_eq!(parse(r#""btn","#).segments.len(), 1);
    }

    #[test]
    fn parses_condition_and_else_branch() {
        let class = parse(r#""on" if enabled else "off""#);
        let condition = class.segments[0].condition.as_ref().unwrap();
        assert!(condition.else_branch.is_some());
    }

    #[test]
    fn rejects_else_without_if() {
        assert!(syn::parse_str::<Class>(r#""a" else "b""#).is_err());
    }

    #[test]
    fn merges_consecutive_literals_into_one_borrowed_str() {
        let out = lower(r#""btn", "btn-lg", "rounded""#);
        assert!(
            out.contains(r#"Cow :: Borrowed ("btn btn-lg rounded")"#),
            "{out}"
        );
    }

    #[test]
    fn empty_literals_vanish_from_the_merge() {
        let out = lower(r#""btn", "", "active""#);
        assert!(out.contains(r#"Cow :: Borrowed ("btn active")"#), "{out}");
    }

    #[test]
    fn dynamic_segments_interrupt_literal_merging() {
        let out = lower(r#""a", extra, "b""#);
        assert!(out.contains(r#"Cow :: Borrowed ("a")"#), "{out}");
        assert!(out.contains(r#"Cow :: Borrowed ("b")"#), "{out}");
        assert!(!out.contains(r#""a b""#), "{out}");
    }

    #[test]
    fn conditional_segment_lowers_to_an_option() {
        let out = lower(r#""active" if is_active"#);
        assert!(out.contains("if is_active"), "{out}");
        assert!(out.contains("Some"), "{out}");
        assert!(out.contains("None"), "{out}");
    }

    #[test]
    fn conditional_literals_do_not_merge() {
        let out = lower(r#""a", "b" if cond"#);
        assert!(out.contains(r#"Cow :: Borrowed ("a")"#), "{out}");
        assert!(!out.contains(r#""a b""#), "{out}");
    }

    #[test]
    fn literal_else_branches_lower_to_a_plain_conditional() {
        let out = lower(r#""on" if enabled else "off""#);
        assert!(!out.contains("ClassBranch"), "{out}");
        assert!(out.contains(r#"Cow :: Borrowed ("on")"#), "{out}");
        assert!(out.contains(r#"Cow :: Borrowed ("off")"#), "{out}");
    }

    #[test]
    fn mixed_else_branches_lower_to_the_branch_enum() {
        let out = lower(r#""on" if enabled else fallback"#);
        assert!(out.contains("ClassBranch :: Then"), "{out}");
        assert!(out.contains("ClassBranch :: Else"), "{out}");
    }

    #[test]
    fn single_entry_skips_the_tuple() {
        let out = lower(r#""btn""#);
        assert!(
            out.contains(r#"Class (:: std :: borrow :: Cow :: Borrowed ("btn"))"#),
            "{out}"
        );
    }

    #[test]
    fn long_lists_nest_tuples_of_twelve() {
        let source = (0..13)
            .map(|i| format!("value{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let out = lower(&source);
        assert!(out.contains("value12"), "{out}");
        assert!(out.contains("(value0 ,"), "{out}");
    }
}
