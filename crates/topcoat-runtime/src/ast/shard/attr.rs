use syn::parse::{Parse, ParseStream};

/// Arguments passed to the `#[shard]` attribute itself. Currently reserved:
/// the macro accepts no arguments today.
pub struct ShardAttr {}

impl Parse for ShardAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}
