use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_context};

mod kw {
    syn::custom_keyword!(with);
}

/// One `with(...)` entry in a route attribute, such as the
/// `with(Head::new())` in `#[page("/about", with(Head::new()))]`.
///
/// The content of the parentheses is kept as written, so the compiler and
/// rust-analyzer see the expression with its original tokens and spans.
pub struct RouteContext {
    pub with: kw::with,
    pub paren: Paren,
    pub value: TokenStream,
}

impl RouteContext {
    /// The `context` method of a route trait impl, inserting the value of
    /// each entry in order, so a later entry shadows an earlier one of the
    /// same type.
    ///
    /// Returns `None` when there are no entries, so the trait's default
    /// applies.
    #[must_use]
    pub fn context_method<'a>(entries: impl IntoIterator<Item = &'a Self>) -> Option<TokenStream> {
        let context = Ident::new("__topcoat_context", Span::mixed_site());
        let mut entries = entries.into_iter().peekable();
        entries.peek()?;
        let inserts = entries.map(|entry| {
            let mut insert = quote_spanned! {entry.with.span=> #context.insert};
            entry
                .paren
                .surround(&mut insert, |tokens| entry.value.to_tokens(tokens));
            quote! { #insert; }
        });
        Some(quote! {
            fn context(&self) -> #topcoat_context::ContextMap {
                let mut #context = #topcoat_context::ContextMap::new();
                #(#inserts)*
                #context
            }
        })
    }
}

impl Parse for RouteContext {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value;
        Ok(Self {
            with: input.parse()?,
            paren: parenthesized!(value in input),
            value: value.parse()?,
        })
    }
}

impl ParseOption for RouteContext {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::with) && input.peek2(Paren)
    }
}

/// Prints the entry as `with(value)`, formatting the value as an expression.
/// A long value breaks onto its own line inside the parentheses.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for RouteContext {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::{BreakMode, Delim};

        printer.move_cursor(self.with.span.start());
        "with".pretty_print(printer);
        printer.move_cursor(self.with.span.end());
        let value = syn::parse2::<syn::Expr>(self.value.clone())
            .unwrap_or_else(|_| syn::Expr::Verbatim(self.value.clone()));
        self.paren
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                value.pretty_print(printer);
            });
    }
}

#[cfg(feature = "pretty")]
impl RouteContext {
    /// Prints the `with(...)` entries that follow the other arguments of a
    /// route attribute, starting with the `comma` that separates them from
    /// those arguments. Each entry goes on its own line when the attribute
    /// breaks. A comma after the last entry is dropped, as `rustfmt` does for
    /// attribute arguments.
    pub(crate) fn pretty_print_entries(
        printer: &mut topcoat_core_grammar::pretty::Printer<'_>,
        comma: Option<&syn::Token![,]>,
        entries: &syn::punctuated::Punctuated<Self, syn::Token![,]>,
    ) {
        use topcoat_core_grammar::pretty::{PrettyPrint, Printer};

        let separator = |printer: &mut Printer<'_>| {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
        };

        if entries.is_empty() {
            return;
        }
        if let Some(comma) = comma {
            comma.pretty_print(printer);
            separator(printer);
        }
        for (index, pair) in entries.pairs().enumerate() {
            pair.value().pretty_print(printer);
            if let Some(punct) = pair.punct()
                && index < entries.len() - 1
            {
                printer.scan_no_break_trivia();
                punct.pretty_print(printer);
                separator(printer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_value_tokens_as_written() {
        // Not an expression, so parsing it would fail.
        let entry: RouteContext = syn::parse_str("with(a, b; c)").unwrap();
        assert_eq!(entry.value.to_string(), quote! { a, b; c }.to_string());
    }

    #[test]
    fn peeks_only_a_with_followed_by_parentheses() {
        let peek = |source: &str| {
            syn::parse::Parser::parse_str(
                |input: ParseStream| {
                    let peeked = RouteContext::peek(input);
                    input.parse::<TokenStream>()?;
                    Ok(peeked)
                },
                source,
            )
            .unwrap()
        };
        assert!(peek("with(value)"));
        assert!(!peek("with"));
        assert!(!peek("with[value]"));
        assert!(!peek("without(value)"));
        assert!(!peek("\"/path\""));
    }

    #[test]
    fn emits_no_method_without_entries() {
        assert!(RouteContext::context_method([]).is_none());
    }
}
