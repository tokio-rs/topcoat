use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::ParseOption;

use crate::{
    template::TemplateBlock,
    view::{
        Component, Nodes,
        hir::{LowerView, ViewBuilder},
    },
};

mod kw {
    syn::custom_keyword!(defer);
}

/// A deferred component and the placeholder rendered until it completes.
pub struct Deferred {
    pub defer_kw: kw::defer,
    pub component: Component,
    pub placeholder: TemplateBlock<Nodes>,
}

impl LowerView for Deferred {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.deferred(
            &self.component.path,
            &self.component.named_args,
            &self.component.children,
            &self.placeholder.children,
            self.defer_kw.span(),
            self.component.paren_token.span.span(),
        );
    }
}

impl Parse for Deferred {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            defer_kw: input.parse()?,
            component: input.parse()?,
            placeholder: input.parse()?,
        })
    }
}

impl ParseOption for Deferred {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::defer)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Deferred {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        printer.move_cursor(self.defer_kw.span().start());
        "defer".pretty_print(printer);
        printer.move_cursor(self.defer_kw.span().end());
        " ".pretty_print(printer);
        self.component.pretty_print(printer);
        " ".pretty_print(printer);
        self.placeholder.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Deferred {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_component_and_placeholder() {
        let deferred = parse(r#"defer activity(label: "recent") { <p>"Loading"</p> }"#);

        assert_eq!(deferred.component.path.segments[0].ident, "activity");
        assert_eq!(deferred.placeholder.children.len(), 1);
    }

    #[test]
    fn emits_a_deferred_view() {
        let deferred = parse(r#"defer activity() { "Loading" }"#);
        let mut builder = ViewBuilder::new();
        deferred.lower(&mut builder);
        let tokens = builder.finish().emit().to_string();

        assert!(tokens.contains("defer"), "{tokens}");
        assert!(tokens.contains("activity"), "{tokens}");
    }
}
