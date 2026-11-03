use proc_macro2::Span;
use syn::{Ident, PatType, Signature};
use topcoat_core_grammar::common::SignatureExt;

/// Checks on the signature of a handler, which takes `cx: &Cx` and then at
/// most one request body parameter.
pub trait HandlerSignatureExt {
    /// Returns the request body parameter, if any.
    ///
    /// # Errors
    ///
    /// Returns an error when more than one parameter follows `cx`.
    fn request_param(&self) -> syn::Result<Option<&PatType>>;
}

impl HandlerSignatureExt for Signature {
    fn request_param(&self) -> syn::Result<Option<&PatType>> {
        let mut params = self.params();
        let request = params.next();
        if let Some(extra) = params.next() {
            return Err(syn::Error::new_spanned(
                extra,
                "handler functions cannot take more than one request body parameter",
            ));
        }
        Ok(request)
    }
}

#[must_use]
pub fn request_ident() -> Ident {
    Ident::new("__topcoat_request", Span::mixed_site())
}
