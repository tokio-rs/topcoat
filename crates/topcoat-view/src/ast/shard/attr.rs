use syn::parse::{Parse, ParseStream};

pub struct ShardAttr {}

impl Parse for ShardAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
