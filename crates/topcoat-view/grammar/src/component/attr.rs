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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for ComponentAttr {
    fn pretty_print(&self, _printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {}
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

    #[cfg(feature = "pretty")]
    #[test]
    fn formats_empty_arguments() {
        use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

        // `rustfmt` lays out the attribute at file level, so it is formatted
        // inside a macro body that `rustfmt` does not reach.
        let mut registry = Registry::new();
        registry
            .register_function_like::<syn::Item>("wrap")
            .register_attribute::<ComponentAttr>("component");
        let output =
            pretty_print_str(&registry, "wrap! { #[component(   )] async fn card() {} }").unwrap();
        assert!(output.contains("#[component()]"), "{output}");
    }
}
