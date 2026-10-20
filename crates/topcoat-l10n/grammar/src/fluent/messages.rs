mod fallback;

pub use fallback::*;

use icu_locale_core::LanguageIdentifier;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Token, Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::fluent::{CallSite, LocaleFiles, LocaleResources, PathPattern};

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
                    self.path.as_ref().map_or_else(Span::call_site, |path| path.lit.span()),
                    format!(
                        "add `fallback = \"...\"` to choose the locale of messages that other \
                         locales lack; found {}",
                        found()
                    ),
                )),
            },
        }
    }

    /// Generates the `msg` module for the files matched by the path from
    /// `call_site`, or compile errors if they cannot be found or parsed.
    #[must_use]
    pub fn expand(&self, call_site: &CallSite) -> TokenStream {
        let path = self.path();
        let locales = match path.resolve(call_site) {
            Ok(locales) => locales,
            Err(err) => return err.to_compile_error(),
        };

        // Including the files makes the compiler rebuild the crate when one of
        // them changes, including after a syntax error.
        let files = locales
            .iter()
            .flat_map(|locale| &locale.files)
            .map(|file| file.to_string_lossy());
        let tracked = quote! {
            #(const _: &str = ::core::include_str!(#files);)*
        };

        let mut errors = Vec::new();
        // TODO: generate the catalog with the fallback.
        let _fallback = match self.fallback(&locales) {
            Ok(fallback) => Some(fallback.clone()),
            Err(err) => {
                errors.push(err.to_compile_error());
                None
            }
        };

        let span = path.lit.span();
        let mut resources = Vec::with_capacity(locales.len());
        for files in locales {
            match LocaleResources::read(files, span) {
                Ok(locale) => resources.push(locale),
                Err(err) => errors.push(err.to_compile_error()),
            }
        }

        let vis = &self.vis;
        // TODO: generate items from `resources`.
        quote! {
            #vis mod msg {
                #tracked
            }
            #(#errors)*
        }
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
}
