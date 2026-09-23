use syn::parse::{Parse, ParseStream};

/// Arguments to `#[shard]`. The attribute accepts none.
pub struct ShardAttr {}

impl Parse for ShardAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
