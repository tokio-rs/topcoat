use std::collections::HashMap;

use proc_macro2::Ident;

#[derive(Default)]
pub(super) struct NameResolver {
    scopes: Vec<HashMap<String, String>>,
    externals: Vec<(Ident, String)>,
    external_by_name: HashMap<String, String>,
    next_local: usize,
    next_external: usize,
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

    pub(super) fn bind_local(&mut self, ident: &Ident, name: String) -> syn::Result<()> {
        let scope = self.scopes.last_mut().ok_or_else(|| {
            syn::Error::new_spanned(ident, "local binding requires an active scope")
        })?;
        scope.insert(ident.to_string(), name);
        Ok(())
    }

    pub(super) fn resolve(&mut self, ident: &Ident) -> String {
        let original = ident.to_string();
        for scope in self.scopes.iter().rev() {
            if let Some(name) = scope.get(&original) {
                return name.clone();
            }
        }

        if let Some(name) = self.external_by_name.get(&original) {
            return name.clone();
        }

        let name = format!("__external{}", self.next_external);
        self.next_external += 1;
        self.external_by_name.insert(original, name.clone());
        self.externals.push((ident.clone(), name.clone()));
        name
    }

    pub(super) fn externals(&self) -> &[(Ident, String)] {
        &self.externals
    }
}
