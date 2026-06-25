use syn::{
    FnArg, ItemFn, Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// The annotated `async fn` that becomes a shard. Validates the function
/// signature: shards must be `async`, must declare a return type, must not
/// take a `self` receiver, and must use identifier patterns for their
/// arguments.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<ShardItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<ShardItem>("async fn counter(cx: &Cx) -> Result {}").unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn counter() -> Result {}").contains("shards must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn counter() {}").contains("must have a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn counter(&self) -> Result {}");
        assert!(err.contains("cannot take a `self` receiver"));
    }

    #[test]
    fn rejects_non_ident_pattern() {
        let err = parse_err("async fn counter((a, b): (u8, u8)) -> Result {}");
        assert!(err.contains("must be identifier patterns"));
    }
}
