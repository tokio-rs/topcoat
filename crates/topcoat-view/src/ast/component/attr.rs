use syn::parse::{Parse, ParseStream};

/// Arguments passed to the `#[component]` attribute itself. Currently
/// reserved: the macro accepts no arguments today.
pub struct ComponentAttr {}

impl Parse for ComponentAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
