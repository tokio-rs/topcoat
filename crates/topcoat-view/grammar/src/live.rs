use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::View;

/// The parsed body of a `live!` invocation: plain Rust statements that emit
/// views through the position's sink with `emit!`.
///
/// Lowers to a [`Live`](topcoat_view::Live) value binding the sink to the
/// `__sink` identifier the nested `emit!` invocations read. The body becomes
/// the async block driving the position verbatim, so it must end in an
/// expression producing the position's `Result<(), Error>`: the final
/// `emit!` itself, an explicit `Ok(())`, or a diverging `loop`.
pub struct Live {
    pub body: Vec<syn::Stmt>,
}

impl Parse for Live {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            body: input.call(syn::Block::parse_within)?,
        })
    }
}

impl ToTokens for Live {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let body = &self.body;
        quote! {
            #topcoat_view::internal::Live::new(
                move |mut __sink: #topcoat_view::internal::LiveSink| async move {
                    #(#body)*
                }
            )
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        for (index, stmt) in self.body.iter().enumerate() {
            stmt.pretty_print(printer);
            if index < self.body.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
        if self.body.len() > 1 {
            printer.scan_force_break();
        }
    }
}

/// The parsed body of an `emit!` invocation: a view body, emitted at the
/// enclosing `live!` invocation's position.
///
/// Lowers to an [`emit`](topcoat_view::LiveSink::emit) call on the sink the
/// enclosing `live!` invocation bound, awaited in place: the invocation is
/// an expression of type `Result<(), Error>`.
pub struct Emit {
    pub view: View,
}

impl Parse for Emit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            view: input.parse()?,
        })
    }
}

impl ToTokens for Emit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let view = &self.view;
        quote! {
            __sink.emit(#view).await
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Emit {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.view.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn live_wraps_the_body_in_a_sink_closure() {
        let live: Live = syn::parse_str("loop { emit! { \"x\" }?; }").unwrap();
        let out = live.to_token_stream().to_string();
        assert!(out.contains("Live :: new"), "{out}");
        assert!(out.contains("| mut __sink"), "{out}");
        assert!(out.contains("async move"), "{out}");
        assert!(out.contains("loop { emit ! { \"x\" } ? ; }"), "{out}");
    }

    #[test]
    fn the_body_tail_expression_is_the_result() {
        let live: Live = syn::parse_str("emit! { \"x\" }").unwrap();
        let out = live.to_token_stream().to_string();
        // No generated `Ok(())`: the tail expression provides the `Result`.
        assert!(!out.contains("Ok (())"), "{out}");
        assert!(out.trim_end().ends_with("emit ! { \"x\" } })"), "{out}");
    }

    #[test]
    fn emit_awaits_a_sink_emission_of_the_view() {
        let emit: Emit = syn::parse_str("<p>\"hi\"</p>").unwrap();
        let out = emit.to_token_stream().to_string();
        assert!(out.starts_with("__sink . emit ("), "{out}");
        assert!(out.ends_with(") . await"), "{out}");
        assert!(out.contains("ViewStream :: new"), "{out}");
    }
}
