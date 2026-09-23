use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::ParseOption;

use crate::common::EndpointPath;

/// The optional endpoint path in `#[shard("/search")]`.
pub struct ShardAttr {
    pub path: Option<EndpointPath>,
}

impl Parse for ShardAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.call(EndpointPath::parse_option)?,
        })
    }
}
