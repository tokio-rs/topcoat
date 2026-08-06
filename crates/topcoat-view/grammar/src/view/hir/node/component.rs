use proc_macro2::Span;
use syn::Path;

use crate::view::{NamedArg, hir::Scope};

/// A component invocation, emitted through the props builder.
pub(crate) struct Component {
    pub path: Path,
    pub named_args: Vec<NamedArg>,
    pub children: Option<Scope>,
    pub span: Span,
}
