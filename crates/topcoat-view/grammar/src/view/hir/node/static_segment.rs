use quote::quote;

use crate::view::hir::emit::{Emit, Emitter};

/// Literal markup, emitted verbatim.
pub(crate) struct StaticSegment {
    pub string: String,
}

impl Emit for StaticSegment {
    fn emit(&self, emitter: &mut Emitter) {
        let string = &self.string;
        emitter.emit(quote! { __unescaped(__cx, __parts, #string); });
    }
}
