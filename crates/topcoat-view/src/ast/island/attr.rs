use syn::parse::{Parse, ParseStream};

pub struct IslandAttr {}

impl Parse for IslandAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
