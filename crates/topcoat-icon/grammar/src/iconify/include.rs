use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Visibility,
    parse::{Parse, ParseStream},
};

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
    pub vis: Visibility,
    pub selection: Selection,
}

impl Include {
    /// Expands to the selected icons. Fails when the icon set is not staged
    /// or does not contain the selected icon.
    fn expand(&self) -> syn::Result<TokenStream> {
        let set = staged_set(&self.selection.prefix, self.selection.span())?;
        Ok(match &self.selection.selected {
            Selected::Set => {
                let doc = format!(
                    "Icons from the `{prefix}` Iconify icon set.",
                    prefix = set.prefix,
                );
                let vis = &self.vis;
                let module = self.selection.module_ident();
                let consts = set_consts(&self.selection, set, &syn::parse_quote!(pub));
                quote! {
                    #[doc = #doc]
                    #vis mod #module {
                        #consts
                    }
                }
            }
            Selected::Glob => set_consts(&self.selection, set, &self.vis),
            Selected::Icon(name) => {
                let icon = resolve_icon(set, name, self.selection.span())?;
                const_item(&self.selection, name, &icon, &self.vis)
            }
        })
    }
}

impl Parse for Include {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            selection: input.parse()?,
        })
    }
}

impl ToTokens for Include {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expand()
            .unwrap_or_else(|error| error.to_compile_error())
            .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Include {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.vis.pretty_print(printer);
        self.selection.pretty_print(printer);
    }
}
