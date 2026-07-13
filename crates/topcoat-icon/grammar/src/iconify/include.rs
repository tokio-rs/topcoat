use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Visibility,
    parse::{Parse, ParseStream},
};

use topcoat_icon::iconify::IconSet;

use crate::iconify::{
    Selected, Selection,
    codegen::{const_item, resolve_icon, set_consts},
    staged::staged_set,
};

/// One `include!` invocation: an optional visibility and a [`Selection`],
/// expanding to `const` icons.
///
/// - `include!("mdi")` expands to a module `mdi` with a `pub const` per icon and alias of the set;
///   the visibility applies to the module.
/// - `include!("mdi:*")` inlines the consts into the current scope; the visibility applies to each
///   one.
/// - `include!("mdi:delete")` expands to the single const `DELETE`.
///
/// Globs skip icons and aliases their set marks as hidden; naming a hidden
/// one explicitly still works.
pub struct Include {
    vis: Visibility,
    selection: Selection,
    set: &'static IconSet,
}

impl Parse for Include {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: Visibility = input.parse()?;
        let selection: Selection = input.parse()?;
        let set = staged_set(&selection.prefix, selection.span())?;
        if let Selected::Icon(name) = &selection.selected {
            // Unknown and unsupported icons are reported while parsing;
            // emission cannot fail afterwards.
            resolve_icon(set, name, selection.span())?;
        }
        Ok(Self {
            vis,
            selection,
            set,
        })
    }
}

impl ToTokens for Include {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self.selection.selected {
            Selected::Set => {
                let doc = format!(
                    "Icons from the `{prefix}` Iconify icon set.",
                    prefix = self.set.prefix,
                );
                let vis = &self.vis;
                let module = self.selection.module_ident();
                let consts = set_consts(&self.selection, self.set, &syn::parse_quote!(pub));
                quote! {
                    #[doc = #doc]
                    #vis mod #module {
                        #consts
                    }
                }
                .to_tokens(tokens);
            }
            Selected::Glob => {
                set_consts(&self.selection, self.set, &self.vis).to_tokens(tokens);
            }
            Selected::Icon(name) => {
                // Validated during parse.
                let Ok(icon) = resolve_icon(self.set, name, self.selection.span()) else {
                    return;
                };
                const_item(&self.selection, name, &icon, &self.vis).to_tokens(tokens);
            }
        }
    }
}
