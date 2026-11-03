use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::common::{
    lenient,
    paths::{topcoat_inventory, topcoat_router},
};

/// Arguments to `segment!`: comma-separated `name: value` options.
///
/// Parsing accepts options that are still being typed, such as a name without a
/// value, so editors can complete them.
pub struct Segment {
    pub options: Punctuated<lenient::FieldValue, Token![,]>,
}

impl Segment {
    /// The option as a field of a `SegmentOptions` struct literal.
    ///
    /// The field name keeps its span, so the compiler reports unknown and repeated
    /// names, and editors complete and document them. The variants of
    /// `SegmentKind` are in scope for the value of `kind`.
    fn option_field(option: &lenient::FieldValue) -> TokenStream {
        let field = option.ident();
        let value = &option.expr;
        if option.name() == "kind" {
            quote! {
                #field: {
                    #[allow(unused_imports)]
                    use #topcoat_router::SegmentKind::*;
                    ::core::option::Option::Some(#value)
                }
            }
        } else {
            quote! {
                #field: ::core::option::Option::Some(::std::borrow::Cow::Borrowed(#value))
            }
        }
    }
}

impl Parse for Segment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            options: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Prints the options on one line if they fit. Otherwise, puts each option on
/// its own line, followed by a comma.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for Segment {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.options.pretty_print(printer);
    }
}

/// Checks the options and, with the `discover` feature, registers the segment.
impl ToTokens for Segment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let fields = self.options.iter().map(Self::option_field);
        let submit = cfg!(feature = "discover").then(|| {
            quote! {
                #topcoat_inventory::submit! {
                    #topcoat_router::Segment::new(module_path!(), OPTIONS)
                }
            }
        });
        quote! {
            const _: () = {
                #[allow(clippy::needless_update)]
                const OPTIONS: #topcoat_router::SegmentOptions = #topcoat_router::SegmentOptions {
                    #(#fields,)*
                    ..#topcoat_router::SegmentOptions::NONE
                };
                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<Segment> {
        syn::parse_str(input)
    }

    fn expand(input: &str) -> String {
        parse(input).unwrap().to_token_stream().to_string()
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn parses_options_being_typed() {
        for input in [
            "",
            "ki",
            "kind:",
            "kind:, rename: \"path\"",
            r#"kind: Param,"#,
        ] {
            assert!(parse(input).is_ok(), "{input}");
        }
    }

    #[test]
    fn rejects_an_equals_sign() {
        assert!(parse("kind = Group").is_err());
    }

    #[test]
    fn writes_each_option_as_a_field() {
        let expanded = expand(r#"kind: CatchAll, rename: NAME"#);
        assert!(expanded.contains(&tokens("Some (CatchAll)")), "{expanded}");
        assert!(expanded.contains(&tokens("Borrowed (NAME)")), "{expanded}");
        assert!(!expanded.contains("compile_error"), "{expanded}");
    }

    #[test]
    fn a_missing_value_is_an_error() {
        for input in ["kind", "kind:"] {
            let expanded = expand(input);
            assert!(expanded.contains("kind :"), "{expanded}");
            assert!(expanded.contains("compile_error"), "{expanded}");
        }
    }
}
