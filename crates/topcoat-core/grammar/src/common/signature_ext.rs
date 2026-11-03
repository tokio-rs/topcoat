use syn::{FnArg, Pat, PatType, Signature, Type, spanned::Spanned};

/// Checks on the signature of a function that receives the request context.
pub trait SignatureExt {
    /// Returns the `cx: &Cx` parameter, which must come first. A function
    /// that does not use the context names it `_cx`.
    ///
    /// # Errors
    ///
    /// Returns an error when the first parameter is not `cx: &Cx`.
    fn cx_param(&self) -> syn::Result<&PatType>;

    /// Returns the parameters after the context. Only the first parameter can
    /// be a `self` receiver, so these are all typed parameters.
    fn params(&self) -> impl Iterator<Item = &PatType>;
}

impl SignatureExt for Signature {
    fn cx_param(&self) -> syn::Result<&PatType> {
        let first = self.inputs.first();
        if let Some(FnArg::Typed(pat_type)) = first
            && let Pat::Ident(pat) = &*pat_type.pat
            && (pat.ident == "cx" || pat.ident == "_cx")
            && is_cx_ref(&pat_type.ty)
        {
            return Ok(pat_type);
        }

        let span = first.map_or_else(|| self.paren_token.span.join(), Spanned::span);
        Err(syn::Error::new(
            span,
            "expected `cx: &Cx` as the first parameter",
        ))
    }

    fn params(&self) -> impl Iterator<Item = &PatType> {
        self.inputs.iter().skip(1).filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => Some(pat_type),
            FnArg::Receiver(_) => None,
        })
    }
}

/// Returns whether `ty` is a shared reference to a type named `Cx`.
fn is_cx_ref(ty: &Type) -> bool {
    let Type::Reference(reference) = ty else {
        return false;
    };
    let Type::Path(path) = &*reference.elem else {
        return false;
    };
    reference.mutability.is_none()
        && path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Cx")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx_param(source: &str) -> syn::Result<()> {
        syn::parse_str::<Signature>(source)?.cx_param().map(drop)
    }

    #[test]
    fn accepts_a_leading_context() {
        assert!(cx_param("fn a(cx: &Cx)").is_ok());
        assert!(cx_param("fn a(_cx: &topcoat::context::Cx, label: &str)").is_ok());
    }

    #[test]
    fn rejects_a_missing_or_misplaced_context() {
        assert!(cx_param("fn a()").is_err());
        assert!(cx_param("fn a(label: &str, cx: &Cx)").is_err());
        assert!(cx_param("fn a(context: &Cx)").is_err());
        assert!(cx_param("fn a(cx: Cx)").is_err());
    }

    #[test]
    fn params_follow_the_context() {
        let sig: Signature = syn::parse_str("fn a(cx: &Cx, label: &str, count: u32)").unwrap();
        assert_eq!(sig.params().count(), 2);
    }
}
