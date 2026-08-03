use quote::quote_spanned;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_view};

use crate::{
    template::TemplateBlock,
    view::{Component, ExprKind, Nodes, ViewWriter, WriteView},
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

impl WriteView for Deferred {
    fn write(&self, writer: &mut ViewWriter) {
        let mut placeholder_writer = ViewWriter::new_nested();
        self.placeholder.write(&mut placeholder_writer);
        let placeholder = placeholder_writer.into_token_stream();
        let component = self.component.render_future();
        let deferred = quote_spanned! {self.defer_kw.span()=>
            #topcoat_view::defer(#placeholder, move |__cx| async move {
                let __cx = __cx.as_ref();
                (#component).await
            })
        };
        writer.write_expr(ExprKind::Node, deferred);
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
        let mut writer = ViewWriter::new();
        deferred.write(&mut writer);
        let tokens = writer.into_token_stream().to_string();

        assert!(tokens.contains("defer"), "{tokens}");
        assert!(tokens.contains("activity"), "{tokens}");
    }
}
