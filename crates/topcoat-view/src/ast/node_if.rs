use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use crate::{
    ast::{NodeBlock, parse_option::ParseOption},
    output::{ViewWriter, ViewWriterIf},
};

/// An `if cond { ... } else { ... }` chain in view-body position.
pub struct NodeIf {
    pub if_token: Token![if],
    pub cond: syn::Expr,
    pub then_branch: NodeBlock,
    pub else_branch: Option<NodeElse>,
}

impl NodeIf {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        let mut writer = writer.begin_if(&self.cond);
        self.then_branch.write(&mut writer);
        if let Some(else_branch) = self.else_branch.as_ref() {
            else_branch.write(writer);
        }
    }
}

impl Parse for NodeIf {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            if_token: input.parse()?,
            cond: input.call(syn::Expr::parse_without_eager_brace)?,
            then_branch: input.parse()?,
            else_branch: input.call(NodeElse::parse_option)?,
        })
    }
}

impl ParseOption for NodeIf {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![if])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeIf {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.if_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.cond.pretty_print(printer);
        " ".pretty_print(printer);
        self.then_branch.pretty_print(printer);
        self.else_branch.pretty_print(printer);
    }
}

/// The trailing `else if ...` or `else { ... }` of a [`NodeIf`].
pub enum NodeElse {
    ElseIf {
        else_token: Token![else],
        node_if: Box<NodeIf>,
    },
    Else {
        else_token: Token![else],
        then_branch: NodeBlock,
    },
}

impl NodeElse {
    fn write(&self, writer: ViewWriterIf<'_>) {
        let mut writer = writer.begin_else();
        match self {
            Self::ElseIf { node_if, .. } => node_if.write(&mut writer),
            Self::Else { then_branch, .. } => then_branch.write(&mut writer),
        }
    }
}

impl Parse for NodeElse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let else_token: Token![else] = input.parse()?;
        if input.peek(Token![if]) {
            Ok(Self::ElseIf {
                else_token,
                node_if: input.parse()?,
            })
        } else {
            Ok(Self::Else {
                else_token,
                then_branch: input.parse()?,
            })
        }
    }
}

impl ParseOption for NodeElse {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![else])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for NodeElse {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::ElseIf {
                else_token,
                node_if,
            } => {
                " ".pretty_print(printer);
                else_token.pretty_print(printer);
                " ".pretty_print(printer);
                node_if.pretty_print(printer);
            }
            Self::Else {
                else_token,
                then_branch,
            } => {
                " ".pretty_print(printer);
                else_token.pretty_print(printer);
                " ".pretty_print(printer);
                then_branch.pretty_print(printer);
            }
        }
    }
}
