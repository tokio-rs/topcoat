use syn::{
    FnArg, ItemFn, Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

pub struct ShardItem {
    item: ItemFn,
}

impl ShardItem {
    pub fn item(&self) -> &ItemFn {
        &self.item
    }
}

impl Parse for ShardItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "shards must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "shards must have a return type",
            ));
        }
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "shard functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(_) => {}
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "shard function arguments must be identifier patterns",
                        ));
                    }
                },
            }
        }
        Ok(Self { item })
    }
}
