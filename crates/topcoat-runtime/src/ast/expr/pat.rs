use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::Pat;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    /// Lowers a binding pattern. The full pattern (including any type
    /// annotation) is emitted to Rust; JavaScript receives a generated local
    /// identifier. Only plain identifiers, optionally annotated with a type,
    /// are supported.
    pub(super) fn pat(
        pat: &Pat,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<(Ident, String)> {
        let inner = match pat {
            Pat::Type(ty) => &*ty.pat,
            other => other,
        };

        let ident = match inner {
            Pat::Ident(ident) if ident.by_ref.is_none() && ident.subpat.is_none() => ident,
            other => return Err(syn::Error::new_spanned(other, "unsupported pattern")),
        };

        let name = names.allocate_local();
        js.push_str(&name);
        pat.to_tokens(rust);
        Ok((ident.ident.clone(), name))
    }
}
