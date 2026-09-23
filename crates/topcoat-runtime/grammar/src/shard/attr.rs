use syn::parse::{Parse, ParseStream};

/// The `#[shard]` attribute, which accepts no arguments.
pub struct ShardAttr {}

impl Parse for ShardAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
