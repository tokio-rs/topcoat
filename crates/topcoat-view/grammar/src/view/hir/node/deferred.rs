use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::Component;
use crate::view::hir::Scope;

/// A deferred component invocation and the placeholder shown until it resolves.
pub(crate) struct Deferred {
    pub component: Component,
    pub placeholder: Scope,
    pub span: Span,
}

impl Deferred {
    pub(in crate::view::hir) fn emit_future(&self) -> TokenStream {
        let component = self.component.emit_future();
        let placeholder = self.placeholder.emit_future();
        let span = self.span;

        quote_spanned! {span=>
            async {
                let __placeholder = (#placeholder).await?;
                ::core::result::Result::<
                    #topcoat_view::View,
                    #topcoat_error::Error,
                >::Ok(#topcoat_view::defer(__placeholder, move |__cx| async move {
                    let __cx = __cx.as_ref();
                    (#component).await
                }))
            }
        }
    }
}
