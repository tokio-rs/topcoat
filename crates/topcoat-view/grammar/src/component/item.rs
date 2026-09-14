use syn::{
    FnArg, ItemFn, Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// The annotated `async fn` that becomes a component. Validates the function
/// signature: components must be `async`, must declare a return type, must
/// not take a `self` receiver, and must use identifier patterns for their
/// arguments.
pub struct ComponentItem {
    item: ItemFn,
}

impl ComponentItem {
    pub fn item(&self) -> &ItemFn {
        &self.item
    }
}

impl Parse for ComponentItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "components must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "components must have a return type",
            ));
        }
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "component functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(_) => {}
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "component function arguments must be identifier patterns",
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
        match syn::parse_str::<ComponentItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<ComponentItem>("async fn badge(label: &str) -> Result {}").unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn badge() -> Result {}").contains("components must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn badge() {}").contains("must have a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn badge(&self) -> Result {}");
        assert!(err.contains("cannot take a `self` receiver"));
    }

    #[test]
    fn rejects_non_ident_pattern() {
        let err = parse_err("async fn badge((a, b): (u8, u8)) -> Result {}");
        assert!(err.contains("must be identifier patterns"));
    }
}
