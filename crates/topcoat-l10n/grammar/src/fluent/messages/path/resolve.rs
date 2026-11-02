use std::{
    collections::{BTreeMap, btree_map::Entry},
    path::PathBuf,
};

use icu_locale::{
    LocaleCanonicalizer,
    fallback::{LocaleFallbackConfig, LocaleFallbacker},
};
use icu_locale_core::{DataLocale, LanguageIdentifier, Locale};
use topcoat_core_grammar::common::CallSite;

use crate::fluent::PathPattern;

/// Files matched for a single locale.
#[derive(Debug)]
pub struct LocaleFiles {
    /// The locale parsed from the `{locale}` match.
    pub locale: LanguageIdentifier,
    /// Matched files as sorted absolute paths.
    pub files: Vec<PathBuf>,
}

impl PathPattern {
    /// Finds matching files and groups them by locale, sorted by canonical locale name.
    ///
    /// # Errors
    ///
    /// Returns an error if no files match, a locale name is invalid, different names
    /// identify the same locale, or a directory cannot be read. Source-relative paths
    /// also fail if `call_site` has no source file.
    pub fn resolve(&self, call_site: &CallSite) -> syn::Result<Vec<LocaleFiles>> {
        let error = |message: String| syn::Error::new(self.lit.span(), message);
        let path = self.lit.value();

        // Grouped by the text matched by `{locale}`. The matches are sorted, so the
        // files of each group are too.
        let mut matches: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for found in self.glob.find(call_site).map_err(error)? {
            let spelling = found
                .capture
                .expect("the pattern has a `{locale}` placeholder");
            matches.entry(spelling).or_default().push(found.path);
        }
        if matches.is_empty() {
            return Err(error(format!("no files match `{path}`")));
        }

        // Keyed by canonical spelling, since language identifiers are not `Ord`.
        let mut locales = BTreeMap::new();
        // The lookup form of each locale, with its first file.
        let mut forms: Vec<(DataLocale, PathBuf)> = Vec::new();
        for (spelling, files) in matches {
            let locale = LanguageIdentifier::try_from_str(&spelling).map_err(|_| {
                error(format!(
                    "`{spelling}` in `{}` is not a valid locale",
                    files[0].display()
                ))
            })?;
            let form = lookup_form(&locale);
            if let Some((_, other)) = forms.iter().find(|(other, _)| *other == form) {
                return Err(error(format!(
                    "`{}` and `{}` are the same locale; use one spelling",
                    other.display(),
                    files[0].display()
                )));
            }
            forms.push((form, files[0].clone()));
            match locales.entry(locale.to_string()) {
                Entry::Vacant(entry) => {
                    entry.insert(LocaleFiles { locale, files });
                }
                Entry::Occupied(entry) => {
                    return Err(error(format!(
                        "`{}` and `{}` are the same locale; use one spelling",
                        entry.get().files[0].display(),
                        files[0].display()
                    )));
                }
            }
        }
        Ok(locales.into_values().collect())
    }
}

/// Normalizes a locale for catalog comparisons by replacing aliases and removing
/// implied subtags. This gives `iw` and `he` the same form, and likewise `zh` and
/// `zh-Hans`.
fn lookup_form(locale: &LanguageIdentifier) -> DataLocale {
    let mut locale = Locale::from(locale.clone());
    LocaleCanonicalizer::new_common().canonicalize(&mut locale);
    LocaleFallbacker::new()
        .for_config(LocaleFallbackConfig::default())
        .fallback_for(DataLocale::from(&locale.id))
        .take()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Creates a test crate containing the given relative file paths.
    fn crate_with(files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        dir
    }

    fn resolve(dir: &TempDir, pattern: &str) -> syn::Result<Vec<LocaleFiles>> {
        let pattern: PathPattern = syn::parse_str(&format!("{pattern:?}")).unwrap();
        let source_file = dir.path().join("src/lib.rs");
        let call_site = CallSite::new(dir.path(), Some(&source_file), None).unwrap();
        pattern.resolve(&call_site)
    }

    #[test]
    fn groups_files_by_locale() {
        let dir = crate_with(&[
            "l10n/en/main.ftl",
            "l10n/en/forms/login.ftl",
            "l10n/de-AT/main.ftl",
        ]);
        let root = std::path::absolute(dir.path()).unwrap();
        let locales = resolve(&dir, "l10n/{locale}/**/*.ftl").unwrap();
        let locales: Vec<_> = locales
            .iter()
            .map(|locale| (locale.locale.to_string(), locale.files.clone()))
            .collect();
        assert_eq!(
            locales,
            [
                ("de-AT".to_owned(), vec![root.join("l10n/de-AT/main.ftl")]),
                (
                    "en".to_owned(),
                    vec![
                        root.join("l10n/en/forms/login.ftl"),
                        root.join("l10n/en/main.ftl"),
                    ]
                ),
            ]
        );
    }

    #[test]
    fn rejects_an_invalid_locale() {
        let dir = crate_with(&["l10n/en.ftl", "l10n/e.ftl"]);
        assert!(resolve(&dir, "l10n/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_two_spellings_of_one_locale() {
        let dir = crate_with(&["l10n/sl-rozaj-biske.ftl", "l10n/sl-biske-rozaj.ftl"]);
        assert!(resolve(&dir, "l10n/{locale}.ftl").is_err());
    }

    /// Equivalent locale names must be rejected, including aliases and names with
    /// implied subtags.
    #[test]
    fn rejects_locales_that_catalogs_consider_the_same() {
        for pair in [["zh", "zh-Hans"], ["iw", "he"], ["en", "en-Latn"]] {
            let files = pair.map(|locale| format!("l10n/{locale}.ftl"));
            let dir = crate_with(&files.each_ref().map(String::as_str));
            assert!(resolve(&dir, "l10n/{locale}.ftl").is_err(), "{pair:?}");
        }
    }

    #[test]
    fn accepts_locales_that_catalogs_keep_apart() {
        for pair in [["zh-Hans", "zh-Hant"], ["en", "en-US"], ["pt", "pt-PT"]] {
            let files = pair.map(|locale| format!("l10n/{locale}.ftl"));
            let dir = crate_with(&files.each_ref().map(String::as_str));
            assert!(resolve(&dir, "l10n/{locale}.ftl").is_ok(), "{pair:?}");
        }
    }

    #[test]
    fn rejects_a_path_without_matches() {
        let dir = crate_with(&["l10n/en.txt"]);
        assert!(resolve(&dir, "l10n/{locale}.ftl").is_err());
        assert!(resolve(&dir, "missing/{locale}.ftl").is_err());
    }
}
