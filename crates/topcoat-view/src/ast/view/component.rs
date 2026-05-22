use quote::quote;
use syn::{
    Expr, Ident, Path, Token, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::ast::{
    ParseOption,
    view::{Nodes, ViewWriter, WriteView},
};

/// A user-defined component invocation, written as
/// `path(name: value, ..., child_node child_node ...)`.
///
/// Named arguments come first, separated by `,`. Any child nodes appear after
/// the last named argument (separated from it by `,`) and run together without
/// separators.
pub struct Component {
    pub path: Path,
    pub paren_token: Paren,
    pub named_args: Vec<NamedArg>,
    pub children: Nodes,
}

/// A `name: value` entry in a component's argument list.
pub struct NamedArg {
    pub ident: Ident,
    pub colon: Token![:],
    pub value: Expr,
}

impl WriteView for Component {
    fn write(&self, writer: &mut ViewWriter) {
        let name = &self.path;
        let fields = self.named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { #ident: #value }
        });
        let mut child_writer = ViewWriter::new_nested();
        for child in &self.children {
            child.write(&mut child_writer);
        }
        let child = child_writer.into_token_stream();

        writer.write_expr(quote! {
            <#name as ::topcoat::view::Component>::render(
                #name { #(#fields),* },
                __cx,
                #child,
            ).await?
        });
    }
}

impl Parse for Component {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            path: input.parse()?,
            paren_token: parenthesized!(content in input),
            named_args: {
                /// Peek whether the stream is positioned at a `name :` named-argument start
                /// (a single colon — `::` would start a path, e.g. `foo::bar()`).
                fn peek_named_arg(input: ParseStream) -> bool {
                    input.peek(Ident) && input.peek2(Token![:]) && !input.peek2(Token![::])
                }

                let mut named_args = Vec::new();
                while !content.is_empty() && peek_named_arg(&content) {
                    let ident: Ident = content.parse()?;
                    let colon: Token![:] = content.parse()?;
                    let value: Expr = content.parse()?;
                    named_args.push(NamedArg {
                        ident,
                        colon,
                        value,
                    });
                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                    } else {
                        break;
                    }
                }

                named_args
            },
            children: content.parse()?,
        })
    }
}

impl ParseOption for Component {
    fn peek(input: ParseStream) -> bool {
        let fork = input.fork();
        fork.parse::<Path>().is_ok() && fork.peek(Paren)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Component {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        printer.scan_begin(topcoat_pretty::BreakMode::Consistent);
        self.path.pretty_print(printer);
        "(".pretty_print(printer);
        let total = self.named_args.len() + self.children.len();
        if total > 0 {
            printer.scan_indent(1);
            printer.scan_break();
            printer.scan_trivia(false, true);

            for (index, arg) in self.named_args.iter().enumerate() {
                arg.ident.pretty_print(printer);
                ": ".pretty_print(printer);
                arg.value.pretty_print(printer);
                let last_named = index == self.named_args.len() - 1;
                if !last_named || !self.children.is_empty() {
                    ",".pretty_print(printer);
                    printer.scan_same_line_trivia();
                    printer.scan_break();
                    " ".pretty_print(printer);
                    printer.scan_trivia(true, true);
                }
            }
            self.children.pretty_print(printer);

            if total > 1 {
                printer.scan_force_break();
            }
            printer.scan_same_line_trivia();
            printer.scan_trivia(true, false);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        ")".pretty_print(printer);
        printer.scan_end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Component {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Component>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn path_segments(component: &Component) -> Vec<String> {
        component
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect()
    }

    #[test]
    fn parses_empty_arg_list() {
        let component = parse("my::widget()");
        assert_eq!(path_segments(&component), vec!["my", "widget"]);
        assert!(component.named_args.is_empty());
        assert!(component.children.is_empty());
    }

    #[test]
    fn parses_children_only() {
        let component = parse(r#"card("hi")"#);
        assert!(component.named_args.is_empty());
        assert_eq!(component.children.len(), 1);
    }

    #[test]
    fn parses_multiple_children_without_separators() {
        let component = parse(r#"card(<div>"foo"</div><div>"bar"</div>)"#);
        assert_eq!(component.children.len(), 2);
    }

    #[test]
    fn parses_named_args_only() {
        let component = parse(r#"button(label: "ok")"#);
        assert_eq!(component.named_args.len(), 1);
        assert_eq!(component.named_args[0].ident.to_string(), "label");
        assert!(component.children.is_empty());
    }

    #[test]
    fn parses_named_args_then_children() {
        let component = parse(r#"button(prop1: 5, prop2: 6, <div>"foo"</div><div>"bar"</div>)"#);
        assert_eq!(component.named_args.len(), 2);
        assert_eq!(component.children.len(), 2);
    }

    #[test]
    fn allows_trailing_comma_when_no_children() {
        let component = parse(r#"button(label: "ok",)"#);
        assert_eq!(component.named_args.len(), 1);
        assert!(component.children.is_empty());
    }

    #[test]
    fn parses_path_qualified_child_component() {
        let component = parse(r#"button(prop1: 5, foo::checkbox())"#);
        assert_eq!(component.named_args.len(), 1);
        assert_eq!(component.children.len(), 1);
    }

    #[test]
    fn named_arg_after_child_is_rejected() {
        assert!(parse_err(r#"button(<div></div> prop1: 5)"#).contains("expected view node"),);
    }
}
