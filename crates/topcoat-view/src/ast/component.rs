use quote::quote;
use syn::{
    Path, Token, bracketed,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Bracket,
};

use crate::{
    ast::{
        Attributes, ComponentClosingTag, ComponentOpeningTag, ComponentSelfClosingTag, Node,
        ParseOption,
    },
    output::ViewWriter,
};

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
    pub fn path(&self) -> &Path {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.path,
            Self::SelfClosing { tag } => &tag.path,
        }
    }

    pub fn attributes(&self) -> &Attributes {
        match self {
            Self::Normal { opening_tag, .. } => &opening_tag.attributes,
            Self::SelfClosing { tag } => &tag.attributes,
        }
    }

    pub fn children(&self) -> &[Node] {
        match self {
            Self::Normal { children, .. } => children,
            Self::SelfClosing { .. } => &[],
        }
    }

    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let name = self.path();
        let fields = self.attributes().items.iter().map(|item| {
            let name = &item.name;
            let value = &item.value;
            quote! { #name: #value }
        });
        let mut child_writer = ViewWriter::new();
        for child in self.children() {
            child.write(&mut child_writer);
        }

        writer.write_expr_unescaped(quote! {
            <#name as ::topcoat::component::Component>::render(#name {
                child: #child_writer,
                #(#fields),*
            }).await
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
impl crate::pretty::PrettyPrint for Component {
    fn pretty_print(&self, printer: &mut crate::pretty::Printer<'_>) {
        printer.scan_begin(crate::pretty::BreakMode::Consistent);
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
