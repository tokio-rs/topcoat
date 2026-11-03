mod component;
mod document_type;
mod element;
mod element_name;
mod element_tag;
mod for_loop;
pub(crate) mod hir;
mod html_ident;
mod node;
mod nodes;

pub use component::*;
pub use document_type::*;
pub use element::*;
pub use element_name::*;
pub use element_tag::*;
pub use for_loop::*;
pub use html_ident::*;
pub use node::*;
pub use nodes::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use crate::view::hir::{LowerView, ViewBuilder};

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::View`](topcoat_view::View).
pub struct View {
    pub nodes: Nodes,
}

impl Parse for View {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            nodes: input.parse()?,
        })
    }
}

impl ToTokens for View {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut builder = ViewBuilder::new();
        self.nodes.lower(&mut builder);
        builder.finish().emit_view().to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for View {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.nodes.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Node;

    fn parse(source: &str) -> View {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn empty_input_yields_no_nodes() {
        assert!(parse("").nodes.is_empty());
    }

    #[test]
    fn collects_sibling_nodes_in_order() {
        let view = parse(r#""a" "b" "c""#);
        assert_eq!(view.nodes.len(), 3);
        assert!(view.nodes.iter().all(|n| matches!(n, Node::Text(_))));
    }

    #[test]
    fn the_view_borrows_the_call_site_cx_without_cloning_it() {
        let tokens = parse("<div></div>").to_token_stream().to_string();
        let capture = tokens.find("async move").expect(&tokens);
        let binding = tokens.find("Cx = & cx ;").expect(&tokens);
        assert!(capture < binding, "{tokens}");
        assert!(!tokens.contains("clone ()"), "{tokens}");
    }

    #[test]
    fn the_view_takes_part_in_the_enclosing_build() {
        let tokens = parse("<div></div>").to_token_stream().to_string();
        assert!(tokens.contains("ScopeView :: new ("), "{tokens}");
        assert!(tokens.contains("MoveView :: drive ("), "{tokens}");
        assert!(!tokens.contains("self_contained"), "{tokens}");
    }

    #[test]
    fn incomplete_expressions_expand_in_view_and_emit() {
        for source in [
            "<p>(value.)</p>",
            "<input value=(value.)>",
            "<input (value.)=\"text\">",
            "<input (value.)>",
            "<input :value=(value.)>",
            "<(value.)></(value.)>",
        ] {
            let view = parse(source);
            let emit = syn::parse_str::<crate::live::Emit>(source).unwrap();
            for expanded in [view.to_token_stream(), emit.to_token_stream()] {
                assert!(expanded.to_string().contains("value ."), "{expanded}");
            }
        }
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn formats_parenthesized_expressions() {
        use topcoat_core_grammar::common::pretty::{Registry, pretty_print_str};

        let mut registry = Registry::new();
        registry.register_function_like::<View>("view");
        registry.register_function_like::<crate::live::Emit>("emit");
        for name in ["view", "emit"] {
            for (input, expected) in [
                ("value . len ()", "value.len()"),
                ("value.", "value."),
                ("value /* keep */ .", "value /* keep */ ."),
            ] {
                let source = format!("{name}! {{ <p>({input})</p> }}");
                let formatted = pretty_print_str(&registry, &source).unwrap();
                assert!(formatted.contains(&format!("({expected})")), "{formatted}");
                assert_eq!(pretty_print_str(&registry, &formatted).unwrap(), formatted);
            }
        }
    }
}
