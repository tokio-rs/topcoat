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
