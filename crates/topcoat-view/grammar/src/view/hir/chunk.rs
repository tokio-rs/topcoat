use proc_macro2::{Span, TokenStream};
use syn::{Expr, Ident, Pat};

use crate::view::ViewWriter;

/// Identifies which `internal` helper a [`Chunk::Expr`] should be wrapped in
/// when emitted, so the generated code uses the matching `__*` function and
/// the corresponding `*ViewParts` trait.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum ExprKind {
    Unescaped,
    Node,
    ElementName,
    Attribute,
    AttributeUnescaped,
    AttributeKey,
    AttributeValue,
    Attributes,
}

impl ExprKind {
    fn helper(self) -> Ident {
        let name = match self {
            Self::Unescaped => "__unescaped",
            Self::Node => "__node",
            Self::ElementName => "__element_name",
            Self::Attribute => "__attribute",
            Self::AttributeUnescaped => "__attribute_unescaped",
            Self::AttributeKey => "__attribute_key",
            Self::AttributeValue => "__attribute_value",
            Self::Attributes => "__attributes",
        };
        Ident::new(name, Span::call_site())
    }
}

pub(super) enum Chunk {
    Static {
        string: String,
    },
    Expr {
        kind: ExprKind,
        tokens: TokenStream,
    },
    Component {
        tokens: TokenStream,
    },
    Local {
        pat: Pat,
        expr: Box<Expr>,
    },
    Statement {
        tokens: TokenStream,
    },
    For {
        pat: Pat,
        expr: Box<Expr>,
        body: Box<ViewWriter>,
    },
    If {
        expr: Expr,
        then_branch: Box<ViewWriter>,
        else_branch: Box<ViewWriter>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Box<ViewWriter>,
}

pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut ViewWriter)) {
        let mut body = ViewWriter::new();
        f(&mut body);
        body.flush();
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: Box::new(body),
        });
    }
}
