use syn::parse::{Parse, ParseStream};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(boxed);
}

/// Arguments passed to the `#[component]` attribute itself. The only
/// recognized argument is `boxed`, which makes the generated `render` return
/// a heap-allocated, type-erased future instead of an opaque one. Recursive
/// components need this on at least one component in the cycle.
pub struct ComponentAttr {
    boxed: Option<kw::boxed>,
}

impl ComponentAttr {
    /// Whether the generated `render` returns a boxed future.
    #[must_use]
    pub fn boxed(&self) -> bool {
        self.boxed.is_some()
    }
}

impl Parse for ComponentAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            boxed: if input.is_empty() {
                None
            } else {
                Some(input.parse()?)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<ComponentAttr>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_empty_arguments() {
        let attr: ComponentAttr = syn::parse_str("").unwrap();
        assert!(!attr.boxed());
    }

    #[test]
    fn parses_boxed() {
        let attr: ComponentAttr = syn::parse_str("boxed").unwrap();
        assert!(attr.boxed());
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(parse_err("pinned").contains("expected `boxed`"));
    }

    #[test]
    fn rejects_trailing_tokens() {
        parse_err("boxed, extra");
    }
}
