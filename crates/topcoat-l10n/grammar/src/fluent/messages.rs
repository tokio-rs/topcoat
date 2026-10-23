mod path;
mod resource;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};

pub use path::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
pub use resource::*;
use syn::{
    Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::paths::{topcoat_inventory, topcoat_l10n, topcoat_l10n_fluent_macro};

/// The input of a `messages!` invocation: an optional visibility and the path
/// of the Fluent files to load.
///
/// ```text
/// messages!(pub(crate) "./l10n/{locale}.ftl")
/// ```
pub struct Messages {
    /// The visibility of the generated `t!`.
    pub vis: Visibility,
    /// The path of the Fluent files.
    pub path: PathPattern,
}

impl Messages {
    /// Returns the name of the hidden `macro_rules!` macro behind `t!`.
    ///
    /// The name contains a hash of the location and the input of the
    /// invocation at `call_site`, so every invocation gets its own name. When
    /// the call site has no source file, as may be the case in rust-analyzer,
    /// the hash also contains a number that grows with every call, so
    /// invocations with the same input still get different names.
    #[must_use]
    pub fn macro_name(&self, call_site: &CallSite) -> Ident {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let start = Span::call_site().start();
        let file = call_site.source_file();
        let counter = file
            .is_none()
            .then(|| COUNTER.fetch_add(1, Ordering::Relaxed));
        let mut hasher = DefaultHasher::new();
        (
            file,
            start.line,
            start.column,
            self.vis.to_token_stream().to_string(),
            self.path.lit.value(),
            counter,
        )
            .hash(&mut hasher);
        format_ident!("__topcoat_l10n_t_{:016x}", hasher.finish())
    }

    /// Generates `t!`: the hidden macro `name` and a `use` that names it `t`
    /// with the visibility of the invocation.
    fn t_macro(&self, name: &Ident) -> TokenStream {
        let vis = &self.vis;
        // Other crates can only call an exported macro, which is placed at the
        // root of the crate, so only a `pub` invocation exports it.
        let export = matches!(vis, Visibility::Public(_)).then(|| quote! { #[macro_export] });
        quote! {
            #[doc(hidden)]
            #export
            #[allow(unused_macros)]
            macro_rules! #name {
                ($($input:tt)*) => {
                    #topcoat_l10n_fluent_macro::__t!(#name; $($input)*)
                };
            }
            #[allow(unused_imports)]
            #vis use #name as t;
        }
    }

    /// Generates the registration of the namespace `name` with the files of
    /// `locales`.
    ///
    /// Including the files also makes the compiler rebuild the crate when one
    /// of them changes.
    fn register(name: &Ident, locales: &[LocaleFiles]) -> TokenStream {
        let id = name.to_string();
        let locales = locales.iter().map(|files| {
            let locale = files.locale.to_string();
            let sources = files.files.iter().map(|file| file.to_string_lossy());
            quote! {
                #topcoat_l10n::NamespaceLocale::new(
                    #locale,
                    &[#(::core::include_str!(#sources)),*],
                )
            }
        });
        quote! {
            #topcoat_inventory::submit! {
                #topcoat_l10n::Namespace::new(#id, &[#(#locales),*])
            }
        }
    }

    /// Generates `t!` and the registration of the files matched by the path
    /// from `call_site`, or compile errors if they cannot be found or parsed.
    #[must_use]
    pub fn expand(&self, call_site: &CallSite) -> TokenStream {
        let name = self.macro_name(call_site);
        let t = self.t_macro(&name);
        let path = &self.path;
        let locales = match path.resolve(call_site) {
            Ok(locales) => locales,
            Err(err) => {
                let err = err.to_compile_error();
                return quote! {
                    #t
                    #err
                };
            }
        };

        // The files are parsed only to report their errors at compile time.
        let span = path.lit.span();
        let errors = locales
            .iter()
            .filter_map(|files| LocaleResources::read(files, span).err())
            .map(|err| err.to_compile_error());
        let register = Self::register(&name, &locales);
        quote! {
            #t
            #register
            #(#errors)*
        }
    }
}

/// Expands to `t!` and the registration of the files found from the
/// [call site](CallSite::current) of the invocation being expanded.
impl ToTokens for Messages {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match CallSite::current() {
            Ok(call_site) => self.expand(&call_site),
            Err(err) => syn::Error::new(Span::call_site(), err).to_compile_error(),
        });
    }
}

impl Parse for Messages {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            path: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use topcoat_core_grammar::testing::assert_deterministic;

    use super::*;

    fn parse(input: &str) -> syn::Result<Messages> {
        syn::parse_str(input)
    }

    #[test]
    fn parses_the_visibility_and_the_path() {
        let messages = parse(r#"pub(crate) "./l10n/{locale}.ftl""#).unwrap();
        assert!(matches!(messages.vis, Visibility::Restricted(_)));
        assert_eq!(messages.path.lit.value(), "./l10n/{locale}.ftl");

        let messages = parse(r#""./l10n/{locale}.ftl""#).unwrap();
        assert!(matches!(messages.vis, Visibility::Inherited));
    }

    #[test]
    fn requires_a_path() {
        assert!(parse("").is_err());
        assert!(parse("pub(crate)").is_err());
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(parse(r#""./l10n/{locale}.ftl" "./other/{locale}.ftl""#).is_err());
        assert!(parse(r#""./l10n/{locale}.ftl", fallback = "de""#).is_err());
    }

    fn call_site(source_file: Option<&str>) -> CallSite {
        CallSite::new(Path::new("/app"), source_file.map(Path::new)).unwrap()
    }

    #[test]
    fn the_macro_name_is_the_same_for_the_same_invocation() {
        let call_site = call_site(Some("/app/src/lib.rs"));
        assert_deterministic(|| {
            parse(r#"pub "./l10n/{locale}.ftl""#).map(|messages| messages.macro_name(&call_site))
        });
    }

    #[test]
    fn the_macro_name_depends_on_the_file_and_the_input() {
        let name = |file, input| parse(input).unwrap().macro_name(&call_site(Some(file)));
        let lib = "/app/src/lib.rs";
        let path = r#""./l10n/{locale}.ftl""#;
        assert_ne!(name(lib, path), name("/app/src/admin.rs", path));
        assert_ne!(
            name(lib, r#"pub "./l10n/{locale}.ftl""#),
            name(lib, r#"pub(crate) "./l10n/{locale}.ftl""#)
        );
        assert_ne!(
            name(lib, r#""./a/{locale}.ftl""#),
            name(lib, r#""./b/{locale}.ftl""#)
        );
    }

    #[test]
    fn invocations_without_a_file_get_different_macro_names() {
        let call_site = call_site(None);
        let name = || {
            parse(r#"pub "./l10n/{locale}.ftl""#)
                .unwrap()
                .macro_name(&call_site)
        };
        assert_ne!(name(), name());
    }
}
