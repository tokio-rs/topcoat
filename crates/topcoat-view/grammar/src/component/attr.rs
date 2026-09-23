use syn::parse::{Parse, ParseStream};

/// Arguments passed to the `#[component]` attribute itself. It takes none.
pub struct ComponentAttr;

impl Parse for ComponentAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self)
        } else {
            Err(input.error("the `component` attribute takes no arguments"))
        }
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
        syn::parse_str::<ComponentAttr>("").unwrap();
    }

    #[test]
    fn rejects_any_argument() {
        assert!(parse_err("boxed").contains("takes no arguments"));
    }
}
