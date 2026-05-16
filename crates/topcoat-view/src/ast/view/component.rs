use quote::quote;
use syn::{
    Path, Token, bracketed,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Bracket,
};

use crate::ast::{
    ParseOption,
    view::{
        AttributeNode, Attributes, ComponentClosingTag, ComponentOpeningTag,
        ComponentSelfClosingTag, Node, ViewWriter, WriteView,
    },
};

/// A user-defined component invocation, written as `[path attr=value]...[/path]`
/// or `[path attr=value /]` for the self-closing form.
pub enum Component {
    Normal {
        opening_tag: ComponentOpeningTag,
        children: Vec<Node>,
        closing_tag: ComponentClosingTag,
    },
    SelfClosing {
        tag: ComponentSelfClosingTag,
    },
}

impl Component {
    /// The component's path expression (e.g. `topcoat::dev::script`).
    pub fn path(&self) -> &Path {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.path,
            Self::SelfClosing { tag } => &tag.path,
        }
    }

    /// The attributes set on the component's opening tag.
    pub fn attributes(&self) -> &Attributes {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.attributes,
            Self::SelfClosing { tag } => &tag.attributes,
        }
    }

    /// The component's children. Always empty for self-closing components.
    pub fn children(&self) -> &[Node] {
        match self {
            Self::Normal { children, .. } => children,
            Self::SelfClosing { .. } => &[],
        }
    }
}

impl WriteView for Component {
    fn write(&self, writer: &mut ViewWriter) {
        let name = self.path();
        let fields = self
            .attributes()
            .items
            .iter()
            .filter_map(|item| match item {
                AttributeNode::Attribute(attr) => {
                    let name = &attr.key;
                    let value = &attr.value;
                    Some(quote! { #name: #value })
                }
                _ => None,
            });
        let mut child_writer = ViewWriter::new_nested();
        for child in self.children() {
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
        let bracket_token = bracketed!(content in input);
        let path: Path = content.parse()?;
        let attributes: Attributes = content.parse()?;

        if content.peek(Token![/]) {
            return Ok(Self::SelfClosing {
                tag: ComponentSelfClosingTag {
                    bracket_token,
                    path,
                    attributes,
                    slash: content.parse()?,
                },
            });
        }

        let opening_tag = ComponentOpeningTag {
            bracket_token,
            path,
            attributes,
        };

        let mut children = Vec::new();
        while !input.is_empty() && !ComponentClosingTag::peek(input) {
            children.push(input.parse()?);
        }

        if input.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                format!(
                    "missing closing tag for opening tag `{}`",
                    &opening_tag.path.segments.last().unwrap().ident
                ),
            ));
        }
        let closing_tag: ComponentClosingTag = input.parse()?;
        if closing_tag.path != opening_tag.path {
            return Err(syn::Error::new(
                closing_tag.path.span(),
                format!(
                    "closing tag `{}` does not match opening tag `{}`",
                    &closing_tag.path.segments.last().unwrap().ident,
                    &opening_tag.path.segments.last().unwrap().ident
                ),
            ));
        }
        Ok(Self::Normal {
            opening_tag,
            children,
            closing_tag,
        })
    }
}

impl ParseOption for Component {
    fn peek(input: ParseStream) -> bool {
        input.peek(Bracket)
    }
}
#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Component {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        printer.scan_begin(topcoat_pretty::BreakMode::Consistent);
        match self {
            Self::Normal {
                opening_tag,
                children,
                closing_tag,
            } => {
                opening_tag.pretty_print(printer);
                printer.scan_indent(1);
                printer.scan_break();
                printer.scan_trivia(false, true);
                for (index, node) in children.iter().enumerate() {
                    node.pretty_print(printer);
                    if index < children.len() - 1 {
                        printer.scan_same_line_trivia();
                        printer.scan_break();
                        " ".pretty_print(printer);
                        printer.scan_trivia(true, true);
                    }
                }
                printer.scan_same_line_trivia();
                printer.scan_trivia(true, false);
                printer.scan_indent(-1);
                printer.scan_break();
                closing_tag.pretty_print(printer);
            }
            Self::SelfClosing { tag } => tag.pretty_print(printer),
        }
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
            .path()
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect()
    }

    #[test]
    fn parses_self_closing_form() {
        let component = parse("[my::widget /]");
        assert!(matches!(component, Component::SelfClosing { .. }));
        assert_eq!(path_segments(&component), vec!["my", "widget"]);
        assert!(component.children().is_empty());
    }

    #[test]
    fn parses_normal_form_with_children() {
        let component = parse(r#"[card]"hi"[/card]"#);
        assert!(matches!(component, Component::Normal { .. }));
        assert_eq!(component.children().len(), 1);
    }

    #[test]
    fn collects_attributes_on_opening_tag() {
        let component = parse(r#"[button label="ok" /]"#);
        let attrs = component.attributes();
        assert_eq!(attrs.items.len(), 1);
        let AttributeNode::Attribute(attr) = &attrs.items[0] else {
            panic!("expected Attribute variant");
        };
        assert_eq!(attr.key.to_string(), "label");
    }

    #[test]
    fn missing_closing_tag_is_rejected() {
        assert!(parse_err("[foo]").contains("missing closing tag"));
    }

    #[test]
    fn mismatched_closing_path_is_rejected() {
        assert!(parse_err("[foo][/bar]").contains("does not match"));
    }
}
