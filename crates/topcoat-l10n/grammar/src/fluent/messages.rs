mod fallback;
mod path;
mod resource;
mod signature;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};

pub use fallback::*;
use icu_locale_core::LanguageIdentifier;
pub use path::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
pub use resource::*;
pub use signature::*;
use syn::{
    Token, Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_l10n, topcoat_l10n_fluent_macro},
};

/// The input of a `messages!` invocation: an optional visibility, an optional
/// path of the Fluent files to load, and an optional fallback locale.
///
/// ```text
/// messages!(pub(crate) "./l10n/{locale}.ftl", fallback = "en")
/// ```
pub struct Messages {
    /// The visibility of the generated module.
    pub vis: Visibility,
    /// The path of the Fluent files, if written.
    pub path: Option<PathPattern>,
    /// The comma after the path, if written.
    pub comma_token: Option<Token![,]>,
    /// The fallback locale, if written.
    pub fallback: Option<MessagesFallback>,
}

impl Messages {
    /// Returns the path of the Fluent files, or the
    /// [default path](PathPattern::default) when none is written.
    #[must_use]
    pub fn path(&self) -> PathPattern {
        self.path.clone().unwrap_or_default()
    }

    /// Returns the fallback locale among the `locales` that have files: the
    /// written one, or the only locale when none is written.
    ///
    /// # Errors
    ///
    /// Fails when the written fallback has no files, or when none is written
    /// and there is more than one locale.
    pub fn fallback<'a>(&self, locales: &'a [LocaleFiles]) -> syn::Result<&'a LanguageIdentifier> {
        let found = || {
            locales
                .iter()
                .map(|files| format!("`{}`", files.locale))
                .collect::<Vec<_>>()
                .join(", ")
        };
        match &self.fallback {
            Some(fallback) => {
                let locale = fallback.locale();
                locales
                    .iter()
                    .map(|files| &files.locale)
                    .find(|candidate| **candidate == locale)
                    .ok_or_else(|| {
                        syn::Error::new(
                            fallback.locale.span(),
                            format!(
                                "the fallback locale `{locale}` has no Fluent files; found {}",
                                found()
                            ),
                        )
                    })
            }
            None => match locales {
                [only] => Ok(&only.locale),
                _ => Err(syn::Error::new(
                    self.path
                        .as_ref()
                        .map_or_else(Span::call_site, |path| path.lit.span()),
                    format!(
                        "add `fallback = \"...\"` to choose the locale of messages that other \
                         locales lack; found {}",
                        found()
                    ),
                )),
            },
        }
    }

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
            self.path().lit.value(),
            counter,
        )
            .hash(&mut hasher);
        format_ident!("__topcoat_l10n_t_{:016x}", hasher.finish())
    }

    /// Generates `t!`: the hidden macro and a `use` that names it `t` with
    /// the visibility of the invocation.
    fn t_macro(&self, call_site: &CallSite) -> TokenStream {
        let name = self.macro_name(call_site);
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

    /// Generates `t!` and the `msg` module for the files matched by the path
    /// from `call_site`, or compile errors if they cannot be found or parsed.
    #[must_use]
    pub fn expand(&self, call_site: &CallSite) -> TokenStream {
        let t = self.t_macro(call_site);
        let path = self.path();
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

        let mut errors = Vec::new();
        let catalog = match self.fallback(&locales) {
            Ok(fallback) => Some(Self::catalog(fallback, &locales)),
            Err(err) => {
                errors.push(err.to_compile_error());
                None
            }
        };

        // Including the files makes the compiler rebuild the crate when one of
        // them changes. The catalog includes them, so they are only included
        // on their own when there is no catalog because of an error.
        let files = locales
            .iter()
            .flat_map(|locale| &locale.files)
            .map(|file| file.to_string_lossy());
        let tracked = quote! {
            #(const _: &str = ::core::include_str!(#files);)*
        };

        let span = path.lit.span();
        let mut resources = Vec::with_capacity(locales.len());
        for files in locales {
            match LocaleResources::read(files, span) {
                Ok(locale) => resources.push(locale),
                Err(err) => errors.push(err.to_compile_error()),
            }
        }

        let signatures = MessageSignature::collect(&resources);
        let messages = match MessageSignature::expand_all(&signatures, span) {
            Ok(messages) => Some(messages),
            Err(err) => {
                errors.push(err.to_compile_error());
                None
            }
        };

        let body = match (catalog, messages) {
            (Some(catalog), Some(messages)) if errors.is_empty() => quote! {
                #catalog
                #messages
            },
            _ => tracked,
        };
        let vis = &self.vis;
        quote! {
            #t
            // Fluent names are often not snake case, such as `loginInput`.
            #[allow(non_snake_case)]
            #vis mod msg {
                #body
            }
            #(#errors)*
        }
    }

    /// Generates the `CATALOG` static, which loads the files of `locales` and
    /// takes missing messages from `fallback`.
    fn catalog(fallback: &LanguageIdentifier, locales: &[LocaleFiles]) -> TokenStream {
        let new_locale = |locale: &LanguageIdentifier| {
            let locale = locale.to_string();
            quote! {
                #topcoat_l10n::Locale::try_from_str(#locale)
                    .expect("the locale is validated by `messages!`")
            }
        };
        let fallback = new_locale(fallback);
        let entries = locales.iter().map(|files| {
            let locale = new_locale(&files.locale);
            let sources = files.files.iter().map(|file| file.to_string_lossy());
            quote! {
                (#locale, &[#(::core::include_str!(#sources)),*][..])
            }
        });
        quote! {
            /// The translations of the messages in this module.
            pub static CATALOG: ::std::sync::LazyLock<#topcoat_l10n::Catalog> =
                ::std::sync::LazyLock::new(|| {
                    #topcoat_l10n::Catalog::new(#fallback, [#(#entries),*])
                });
        }
    }
}

/// Expands to the `msg` module, with the files found from the
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
        let vis = input.parse()?;
        let path = input.call(PathPattern::parse_option)?;
        // A written path is separated from the fallback by a comma.
        let comma_token = match path {
            Some(_) => input.parse()?,
            None => None,
        };
        let fallback = if path.is_none() || comma_token.is_some() {
            input.call(MessagesFallback::parse_option)?
        } else {
            None
        };
        Ok(Self {
            vis,
            path,
            comma_token,
            fallback,
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
    fn uses_the_default_path_when_none_is_written() {
        let messages = parse("pub(crate)").unwrap();
        assert!(matches!(messages.vis, Visibility::Restricted(_)));
        assert_eq!(
            messages.path().lit.value(),
            PathPattern::default().lit.value()
        );
        assert!(parse("").is_ok());
    }

    #[test]
    fn uses_the_written_path() {
        let messages = parse(r#""./l10n/{locale}.ftl""#).unwrap();
        assert!(matches!(messages.vis, Visibility::Inherited));
        assert_eq!(messages.path().lit.value(), "./l10n/{locale}.ftl");
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(parse(r#""./l10n/{locale}.ftl" "./other/{locale}.ftl""#).is_err());
    }

    #[test]
    fn parses_a_fallback_after_the_path() {
        let messages = parse(r#"pub "./l10n/{locale}.ftl", fallback = "de""#).unwrap();
        assert!(messages.path.is_some());
        assert!(messages.fallback.is_some());
    }

    #[test]
    fn parses_a_fallback_without_a_path() {
        let messages = parse(r#"pub fallback = "de""#).unwrap();
        assert!(matches!(messages.vis, Visibility::Public(_)));
        assert!(messages.path.is_none());
        assert!(messages.fallback.is_some());
    }

    #[test]
    fn accepts_a_trailing_comma_after_the_path() {
        let messages = parse(r#""./l10n/{locale}.ftl","#).unwrap();
        assert!(messages.fallback.is_none());
    }

    #[test]
    fn rejects_a_fallback_without_a_comma_after_the_path() {
        assert!(parse(r#""./l10n/{locale}.ftl" fallback = "de""#).is_err());
    }

    fn locales(locales: &[&str]) -> Vec<LocaleFiles> {
        locales
            .iter()
            .map(|locale| LocaleFiles {
                locale: LanguageIdentifier::try_from_str(locale).unwrap(),
                files: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn uses_the_written_fallback() {
        let messages = parse(r#"fallback = "de""#).unwrap();
        let locales = locales(&["de", "en"]);
        let fallback = messages.fallback(&locales).unwrap();
        assert_eq!(fallback.to_string(), "de");
    }

    #[test]
    fn rejects_a_fallback_without_files() {
        let messages = parse(r#"fallback = "fr""#).unwrap();
        assert!(messages.fallback(&locales(&["de", "en"])).is_err());
    }

    #[test]
    fn falls_back_to_the_only_locale() {
        let messages = parse("").unwrap();
        let locales = locales(&["en"]);
        let fallback = messages.fallback(&locales).unwrap();
        assert_eq!(fallback.to_string(), "en");
    }

    #[test]
    fn requires_a_fallback_for_several_locales() {
        let messages = parse("").unwrap();
        assert!(messages.fallback(&locales(&["de", "en"])).is_err());
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
        assert_ne!(name(lib, "pub"), name("/app/src/admin.rs", "pub"));
        assert_ne!(name(lib, "pub"), name(lib, "pub(crate)"));
        assert_ne!(
            name(lib, r#""./a/{locale}.ftl""#),
            name(lib, r#""./b/{locale}.ftl""#)
        );
    }

    #[test]
    fn invocations_without_a_file_get_different_macro_names() {
        let call_site = call_site(None);
        let name = || parse("pub").unwrap().macro_name(&call_site);
        assert_ne!(name(), name());
    }
}
