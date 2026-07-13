use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_context;

/// The leading `cx =>` argument naming the request context a macro body
/// renders against.
///
/// Inside a `#[component]`, `#[page]`, or `#[layout]`, the context is
/// available implicitly and the argument is omitted. Anywhere else the caller
/// names it explicitly, as in `view! { cx => ... }`.
///
/// An identifier followed by `=>` can never begin an expression or markup, so
/// the argument only assigns meaning to input that would otherwise be
/// rejected. This holds for every macro that shares the convention, including
/// `class!`, whose arguments are arbitrary comma-separated expressions.
/// Matching only an identifier (rather than an arbitrary expression) keeps
/// parsing cheap and unambiguous.
pub struct LeadingCx {
    pub cx: Ident,
    pub fat_arrow_token: Token![=>],
}

impl Parse for LeadingCx {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cx: input.parse()?,
            fat_arrow_token: input.parse()?,
        })
    }
}

impl ParseOption for LeadingCx {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident) && input.peek2(Token![=>])
    }
}

impl ToTokens for LeadingCx {
    /// Emits the `let` statement binding the named context to the `__cx`
    /// identifier the surrounding generated code reads from.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cx = &self.cx;
        quote! { let __cx: &#topcoat_context::Cx = #cx; }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LeadingCx {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.cx.pretty_print(printer);
        " =>".pretty_print(printer);
        // Break after the `cx =>` argument just like between sibling items, so
        // it sits on its own line when the body is laid out multiline.
        printer.scan_same_line_trivia();
        printer.scan_break();
        " ".pretty_print(printer);
        printer.scan_trivia(true, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ident_and_fat_arrow() {
        let leading: LeadingCx = syn::parse_str("cx =>").unwrap();
        assert_eq!(leading.cx.to_string(), "cx");
    }

    #[test]
    fn peek_rejects_other_leading_tokens() {
        fn peek(source: &str) -> bool {
            struct Peek(bool);
            impl Parse for Peek {
                fn parse(input: ParseStream) -> syn::Result<Self> {
                    let peeked = LeadingCx::peek(input);
                    input.parse::<TokenStream>()?;
                    Ok(Self(peeked))
                }
            }
            syn::parse_str::<Peek>(source).unwrap().0
        }

        assert!(peek("cx => <div></div>"));
        assert!(!peek("cx, <div></div>"));
        assert!(!peek(r#"greeting(name: "World")"#));
        assert!(!peek(r#""btn", "active""#));
    }

    #[test]
    fn to_tokens_binds_the_context_identifier() {
        let leading: LeadingCx = syn::parse_str("my_cx =>").unwrap();
        let tokens = leading.to_token_stream().to_string();
        assert!(tokens.contains("let __cx"), "{tokens}");
        assert!(tokens.contains("my_cx"), "{tokens}");
    }
}
