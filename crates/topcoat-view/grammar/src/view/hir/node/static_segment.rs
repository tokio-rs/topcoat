use quote::quote;

use crate::view::hir::emit::{Emit, Emitter};

/// Literal markup, emitted verbatim.
pub(crate) struct StaticSegment {
    pub string: String,
}

impl Emit for StaticSegment {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let string = &self.string;
        emitter.push(quote! { __b.markup(&#string); });
    }
}
