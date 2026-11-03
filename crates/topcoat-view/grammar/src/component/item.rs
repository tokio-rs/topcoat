use syn::{
    Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::common::{SignatureExt, lenient};

/// The annotated `async fn` that becomes a component. Validates the function
/// signature: components must be `async`, must declare a return type, must
/// take `cx: &Cx` first, and must use identifier patterns for their other
/// arguments.
pub struct ComponentItem {
    item: lenient::ItemFn,
}

impl ComponentItem {
    #[must_use]
    pub fn item(&self) -> &lenient::ItemFn {
        &self.item
    }
}

impl Parse for ComponentItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: lenient::ItemFn = input.parse()?;
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
        item.sig.cx_param()?;
        for pat_type in item.sig.params() {
            if !matches!(&*pat_type.pat, Pat::Ident(_)) {
                return Err(syn::Error::new_spanned(
                    pat_type,
                    "component function arguments must be identifier patterns",
                ));
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
        syn::parse_str::<ComponentItem>("async fn badge(cx: &Cx, label: &str) -> Result {}")
            .unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn badge(cx: &Cx) -> Result {}").contains("components must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn badge(cx: &Cx) {}").contains("must have a return type"));
    }

    #[test]
    fn rejects_missing_context() {
        assert!(parse_err("async fn badge(label: &str) -> Result {}").contains("cx: &Cx"));
    }

    #[test]
    fn rejects_non_ident_pattern() {
        let err = parse_err("async fn badge(cx: &Cx, (a, b): (u8, u8)) -> Result {}");
        assert!(err.contains("must be identifier patterns"));
    }
}
