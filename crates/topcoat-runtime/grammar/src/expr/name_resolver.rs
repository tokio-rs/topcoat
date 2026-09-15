use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use topcoat_core_grammar::paths::topcoat_runtime;

pub(super) enum ResolvedIdent {
    Local { js_name: String, rust_ident: Ident },
    External { js_name: String, rust_ident: Ident },
}

struct LocalBinding {
    js_name: String,
    rust_ident: Ident,
    kind: LocalBindingKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalBindingKind {
    Plain,
    Surrogate,
}

pub(super) struct ExternalBinding {
    pub(super) value: TokenStream,
    pub(super) rust_ident: Ident,
    pub(super) js_name: String,
}

#[derive(Default)]
pub(super) struct NameResolver {
    scopes: Vec<HashMap<String, LocalBinding>>,
    externals: Vec<ExternalBinding>,
    external_by_name: HashMap<String, usize>,
    next_local: usize,
}

impl NameResolver {
    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn allocate_local(&mut self) -> String {
        let name = format!("__local{}", self.next_local);
        self.next_local += 1;
        name
    }

    pub(super) fn bind_local(
        &mut self,
        ident: &Ident,
        name: String,
        kind: LocalBindingKind,
    ) -> syn::Result<()> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            syn::Error::new_spanned(ident, "local binding requires an active scope")
        })?;
        scope.insert(
            ident.to_string(),
            LocalBinding {
                js_name: name,
                rust_ident: ident.clone(),
                kind,
            },
        );
        Ok(())
    }

    pub(super) fn resolve(&mut self, ident: &Ident) -> ResolvedIdent {
        let original = ident.to_string();
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(&original) {
                return ResolvedIdent::Local {
                    js_name: binding.js_name.clone(),
                    rust_ident: binding.rust_ident.clone(),
                };
            }
        }

        if let Some(index) = self.external_by_name.get(&original) {
            let binding = &self.externals[*index];
            return ResolvedIdent::External {
                js_name: binding.js_name.clone(),
                rust_ident: binding.rust_ident.clone(),
            };
        }

        let index = self.externals.len();
        let (rust_ident, js_name) = self.capture_value(
            quote! {
                #topcoat_runtime::Surrogated::into_surrogate(
                    ::core::clone::Clone::clone(&#ident),
                )
            },
            ident.span(),
        );
        self.external_by_name.insert(original, index);
        ResolvedIdent::External {
            js_name,
            rust_ident,
        }
    }

    /// Binds a value serialized by the Rust target when the expression is built.
    pub(super) fn capture_value(&mut self, value: TokenStream, span: Span) -> (Ident, String) {
        let index = self.externals.len();
        let js_name = format!("__external{index}");
        let rust_ident = Ident::new(&format!("__topcoat_external{index}"), span);
        self.externals.push(ExternalBinding {
            value,
            js_name: js_name.clone(),
            rust_ident: rust_ident.clone(),
        });
        (rust_ident, js_name)
    }

    pub(super) fn is_surrogate_local(&self, ident: &Ident) -> bool {
        let original = ident.to_string();
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(&original) {
                return binding.kind == LocalBindingKind::Surrogate;
            }
        }
        false
    }

    pub(super) fn externals(&self) -> &[ExternalBinding] {
        &self.externals
    }
}
