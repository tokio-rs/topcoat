use syn::parse::{Parse, ParseStream};

/// The arguments of the `#[shard]` attribute, which takes none.
pub struct ShardAttr {}

impl Parse for ShardAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
