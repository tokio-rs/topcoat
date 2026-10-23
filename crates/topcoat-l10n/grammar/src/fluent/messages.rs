mod path;
mod resource;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
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

/// Arguments to `messages!`: a file pattern, optionally preceded by a visibility.
///
/// ```text
/// messages!(pub(crate) "./l10n/{locale}.ftl")
/// ```
pub struct Messages {
    /// Visibility to apply to the generated `t!` macro.
    pub vis: Visibility,
    /// The file pattern to load.
    pub path: PathPattern,
}

impl Messages {
    /// Chooses a unique name for the hidden `macro_rules!` macro used by `t!`.
    ///
    /// The name includes a hash of the call's location and input. If `call_site` has no
    /// source file, as can happen in rust-analyzer, a counter also distinguishes calls
    /// with identical input.
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

    /// Generates the hidden macro `name` and imports it as `t` with the requested
    /// visibility.
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

    /// Generates the `t` module for namespace `name` with the requested visibility.
    fn t_module(&self, name: &Ident) -> TokenStream {
        let vis = &self.vis;
        let id = name.to_string();
        let doc = format!(
            "The Fluent messages loaded from `{}`.",
            self.path.lit.value()
        );
        quote! {
            #[doc = #doc]
            #vis mod t {
                /// Lists the loaded locales in canonical name order.
                #[allow(dead_code)]
                #[must_use]
                pub fn locales() -> &'static [#topcoat_l10n::Locale] {
                    #topcoat_l10n::Namespace::locales(#id)
                }
            }
        }
    }

    /// Generates the registration for namespace `name`, its locale files and a record
    /// of `call_site` for tools.
    ///
    /// The file includes also tell the compiler to rebuild when a loaded file changes.
    fn register(&self, name: &Ident, call_site: &CallSite, locales: &[LocaleFiles]) -> TokenStream {
        let id = name.to_string();
        let source_file = call_site
            .source_file()
            .map(Path::to_string_lossy)
            .unwrap_or_default();
        let start = Span::call_site().start();
        let line = u32::try_from(start.line).unwrap_or(0);
        let column = u32::try_from(start.column + 1).unwrap_or(0);
        let pattern = self.path.lit.value();
        let record_locales = locales.iter().map(|files| {
            let locale = files.locale.to_string();
            let paths = files.files.iter().map(|file| file.to_string_lossy());
            quote! { (#locale, &[#(#paths),*]) }
        });
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
                #topcoat_l10n::Namespace::new(
                    {
                        const LOCALES: &[(&str, &[&str])] = &[#(#record_locales),*];
                        const LEN: usize = #topcoat_l10n::RawNamespace::encoded_len(
                            #id,
                            #source_file,
                            #pattern,
                            LOCALES,
                        );
                        static RECORD: [u8; LEN] = #topcoat_l10n::RawNamespace::encode(
                            #id,
                            #source_file,
                            #line,
                            #column,
                            #pattern,
                            LOCALES,
                        );
                        &RECORD
                    },
                    &[#(#locales),*],
                )
            }
        }
    }

    /// Generates the translation macro, locale module and registration for files
    /// matched from `call_site`. Reports compile errors if the files cannot be found or
    /// parsed.
    ///
    /// If no source file is available, as can happen in rust-analyzer, generates only
    /// `t!` and the `t` module without reading files.
    #[must_use]
    pub fn expand(&self, call_site: &CallSite) -> TokenStream {
        let name = self.macro_name(call_site);
        let t_macro = self.t_macro(&name);
        let t_module = self.t_module(&name);
        let t = quote! {
            #t_macro
            #t_module
        };
        // The files are only needed when the program runs, and rustc knows the
        // source file of every invocation written in the crate. Without one,
        // a relative path cannot be resolved.
        if call_site.source_file().is_none() {
            return t;
        }
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
        let register = self.register(&name, call_site, &locales);
        quote! {
            #t
            #register
            #(#errors)*
        }
    }
}

/// Generates `t!`, the `t` module and file registration using the current [call
/// site](CallSite::current).
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
    fn reads_no_files_without_a_source_file() {
        let messages = parse(r#""./missing/{locale}.ftl""#).unwrap();
        // With a source file, the missing files are an error.
        let with_file = messages
            .expand(&call_site(Some("/app/src/lib.rs")))
            .to_string();
        assert!(with_file.contains("compile_error"), "{with_file}");
        // Without one, the files are not looked for.
        let without_file = messages.expand(&call_site(None)).to_string();
        assert!(!without_file.contains("compile_error"), "{without_file}");
        assert!(without_file.contains("macro_rules"), "{without_file}");
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
