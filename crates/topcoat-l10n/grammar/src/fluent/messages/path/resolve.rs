use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    ffi::OsStr,
    fs::{self, FileType},
    io,
    path::{Path, PathBuf},
};

use icu_locale_core::LanguageIdentifier;

use crate::fluent::{PathComponent, PathPattern};

/// The directories that hold the root files of a package's binaries, tests,
/// examples and benchmarks, relative to its manifest directory.
const TARGET_DIRS: [&str; 4] = ["src/bin", "tests", "examples", "benches"];

/// The location of a macro invocation, which anchors the relative paths of a
/// [`PathPattern`].
pub struct CallSite {
    manifest_dir: PathBuf,
    source_file: Option<PathBuf>,
}

impl CallSite {
    /// Creates a call site in the crate whose `Cargo.toml` is in
    /// `manifest_dir`, invoked from `source_file` if it is a file on disk.
    /// Relative paths are taken relative to the current directory.
    ///
    /// # Errors
    ///
    /// Fails when the current directory is needed but cannot be read.
    pub fn new(manifest_dir: &Path, source_file: Option<&Path>) -> io::Result<Self> {
        Ok(Self {
            manifest_dir: std::path::absolute(manifest_dir)?,
            source_file: source_file.map(std::path::absolute).transpose()?,
        })
    }

    /// Returns the directory of the invoking module, where its child modules
    /// live.
    ///
    /// That is the directory of the source file for crate roots and `mod.rs`
    /// files, and a directory named after the file next to it otherwise.
    fn module_dir(&self, source_file: &Path) -> Option<PathBuf> {
        let dir = source_file.parent()?;
        let stem = source_file.file_stem()?;
        if stem == "mod" || self.is_crate_root(dir, stem) {
            Some(dir.to_owned())
        } else {
            Some(dir.join(stem))
        }
    }

    /// Returns whether the file named `stem` in `dir` is the root file of one of the
    /// package's targets, judged by Cargo's default target layout.
    fn is_crate_root(&self, dir: &Path, stem: &OsStr) -> bool {
        let is_target_dir = |dir: &Path| {
            TARGET_DIRS
                .iter()
                .any(|target_dir| dir == self.manifest_dir.join(target_dir))
        };
        is_target_dir(dir)
            || ((stem == "lib" || stem == "main") && dir == self.manifest_dir.join("src"))
            || (stem == "main" && dir.parent().is_some_and(is_target_dir))
    }
}

/// The Fluent files of one locale.
#[derive(Debug)]
pub struct LocaleFiles {
    /// The locale, as matched by the `{locale}` placeholder.
    pub locale: LanguageIdentifier,
    /// The absolute paths of the files, sorted.
    pub files: Vec<PathBuf>,
}

impl PathPattern {
    /// Finds the files matched by the path and groups them by locale.
    ///
    /// The locales are sorted by their canonical spelling. Wildcards do not
    /// match names starting with `.` unless the pattern does, and `**` does not
    /// enter symbolic links to directories.
    ///
    /// # Errors
    ///
    /// Fails when no file matches, a matched locale is invalid, two matched
    /// locales are spelled differently but are the same, the path is relative
    /// to the source file but `call_site` has none, or a directory cannot be
    /// read.
    pub fn resolve(&self, call_site: &CallSite) -> syn::Result<Vec<LocaleFiles>> {
        let error = |message: String| syn::Error::new(self.lit.span(), message);
        let path = self.lit.value();
        let components = self.components();

        let source_file = || {
            call_site.source_file.as_deref().ok_or_else(|| {
                error(format!(
                    "cannot resolve `{path}` because the invoking source file is unknown"
                ))
            })
        };
        let (base, components) = match components.split_first() {
            Some((PathComponent::Module, rest)) => {
                let dir = call_site.module_dir(source_file()?).ok_or_else(|| {
                    error("cannot find the directory of the invoking module".to_owned())
                })?;
                (dir, rest)
            }
            _ if path.starts_with('/') => (PathBuf::from("/"), &components[..]),
            _ if path.starts_with("./") || path.starts_with("../") => {
                let dir = source_file()?.parent().unwrap_or(Path::new("/"));
                (dir.to_owned(), &components[..])
            }
            _ => (call_site.manifest_dir.clone(), &components[..]),
        };

        let mut matches = Matches::new();
        walk(&base, components, None, &mut matches).map_err(|err| error(err.to_string()))?;
        if matches.is_empty() {
            return Err(error(format!(
                "no files match `{path}` in `{}`",
                base.display()
            )));
        }

        // Keyed by canonical spelling, since language identifiers are not `Ord`.
        let mut locales = BTreeMap::new();
        for (spelling, files) in matches {
            let files = Vec::from_iter(files);
            let locale = LanguageIdentifier::try_from_str(&spelling).map_err(|_| {
                error(format!(
                    "`{spelling}` in `{}` is not a valid locale",
                    files[0].display()
                ))
            })?;
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

/// Matched files, keyed by the text of the `{locale}` placeholder.
type Matches = BTreeMap<String, BTreeSet<PathBuf>>;

/// Matches `components` against the entries of `dir` and adds each matching
/// file to `matches`. `locale` is the text matched by the `{locale}`
/// placeholder so far.
fn walk(
    dir: &Path,
    components: &[PathComponent],
    locale: Option<&str>,
    matches: &mut Matches,
) -> io::Result<()> {
    let Some((component, rest)) = components.split_first() else {
        if let Some(locale) = locale
            && dir.is_file()
        {
            matches
                .entry(locale.to_owned())
                .or_default()
                .insert(dir.to_owned());
        }
        return Ok(());
    };

    match component {
        PathComponent::Module => {
            unreachable!("`{{mod}}` is only the first component, which is the base")
        }
        PathComponent::Literal(name) if name.is_empty() || name == "." => {
            walk(dir, rest, locale, matches)
        }
        PathComponent::Literal(name) if name == ".." => {
            walk(dir.parent().unwrap_or(dir), rest, locale, matches)
        }
        PathComponent::Literal(name) => walk(&dir.join(name), rest, locale, matches),
        PathComponent::Wildcard(pattern) => {
            for (name, _) in entries(dir)? {
                if is_visible(&name, pattern) && matches_wildcard(pattern, &name) {
                    walk(&dir.join(name), rest, locale, matches)?;
                }
            }
            Ok(())
        }
        PathComponent::Recursive => {
            walk(dir, rest, locale, matches)?;
            for (name, file_type) in entries(dir)? {
                if file_type.is_dir() && is_visible(&name, "") {
                    walk(&dir.join(name), components, locale, matches)?;
                }
            }
            Ok(())
        }
        PathComponent::Locale { prefix, suffix } => {
            for (name, _) in entries(dir)? {
                let found = name
                    .strip_prefix(prefix.as_str())
                    .and_then(|name| name.strip_suffix(suffix.as_str()));
                if let Some(found) = found
                    && !found.is_empty()
                    && is_visible(&name, prefix)
                {
                    walk(&dir.join(&name), rest, Some(found), matches)?;
                }
            }
            Ok(())
        }
    }
}

/// Returns the names and types of the entries of `dir`, sorted by name.
/// Returns nothing when `dir` is not a directory, and skips names that are not
/// valid UTF-8.
fn entries(dir: &Path) -> io::Result<Vec<(String, FileType)>> {
    let error = |err: io::Error| {
        io::Error::new(
            err.kind(),
            format!("failed to read `{}`: {err}", dir.display()),
        )
    };
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(Vec::new());
        }
        Err(err) => return Err(error(err)),
    };
    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(error)?;
        if let Ok(name) = entry.file_name().into_string() {
            entries.push((name, entry.file_type().map_err(error)?));
        }
    }
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    Ok(entries)
}

/// Returns whether a wildcard matches `name`: names starting with `.` only
/// match a pattern that starts with `.` too.
fn is_visible(name: &str, pattern: &str) -> bool {
    !name.starts_with('.') || pattern.starts_with('.')
}

/// Returns whether `name` matches `pattern`, where each `*` matches any
/// characters.
fn matches_wildcard(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(mut rest) = parts.next().and_then(|first| name.strip_prefix(first)) else {
        return false;
    };
    let Some(last) = parts.next_back() else {
        return rest.is_empty();
    };
    let Some(middle) = rest.strip_suffix(last) else {
        return false;
    };
    rest = middle;
    for part in parts {
        let Some(index) = rest.find(part) else {
            return false;
        };
        rest = &rest[index + part.len()..];
    }
    true
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// A crate directory with the given files, each relative to its root.
    fn crate_with(files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        dir
    }

    fn resolve(
        dir: &TempDir,
        source_file: Option<&str>,
        pattern: &str,
    ) -> syn::Result<Vec<LocaleFiles>> {
        let pattern: PathPattern = syn::parse_str(&format!("{pattern:?}")).unwrap();
        let source_file = source_file.map(|file| dir.path().join(file));
        let call_site = CallSite::new(dir.path(), source_file.as_deref()).unwrap();
        pattern.resolve(&call_site)
    }

    /// Resolves `pattern` and lists each locale with its files relative to
    /// the crate directory.
    fn resolved(
        dir: &TempDir,
        source_file: Option<&str>,
        pattern: &str,
    ) -> Vec<(String, Vec<String>)> {
        let root = std::path::absolute(dir.path()).unwrap();
        resolve(dir, source_file, pattern)
            .unwrap()
            .into_iter()
            .map(|locale| {
                let files = locale
                    .files
                    .iter()
                    .map(|file| {
                        let relative = file.strip_prefix(&root).unwrap();
                        relative.to_str().unwrap().replace('\\', "/")
                    })
                    .collect();
                (locale.locale.to_string(), files)
            })
            .collect()
    }

    fn expected(locales: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        locales
            .iter()
            .map(|(locale, files)| {
                let files = files.iter().map(|file| (*file).to_owned()).collect();
                ((*locale).to_owned(), files)
            })
            .collect()
    }

    #[test]
    fn resolves_plain_paths_from_the_manifest_directory() {
        let dir = crate_with(&["l10n/en.ftl", "l10n/de-AT.ftl", "src/l10n/fr.ftl"]);
        assert_eq!(
            resolved(&dir, Some("src/lib.rs"), "l10n/{locale}.ftl"),
            expected(&[("de-AT", &["l10n/de-AT.ftl"]), ("en", &["l10n/en.ftl"])]),
        );
    }

    #[test]
    fn resolves_dot_paths_from_the_source_file_directory() {
        let dir = crate_with(&["l10n/fr.ftl", "src/pages/l10n/en.ftl", "src/shared/de.ftl"]);
        assert_eq!(
            resolved(&dir, Some("src/pages/about.rs"), "./l10n/{locale}.ftl"),
            expected(&[("en", &["src/pages/l10n/en.ftl"])]),
        );
        assert_eq!(
            resolved(&dir, Some("src/pages/about.rs"), "../shared/{locale}.ftl"),
            expected(&[("de", &["src/shared/de.ftl"])]),
        );
    }

    #[test]
    fn resolves_absolute_paths_as_written() {
        let dir = crate_with(&["l10n/en.ftl"]);
        let other = crate_with(&["l10n/de.ftl"]);
        let root = std::path::absolute(other.path()).unwrap();
        let pattern = format!("{}/l10n/{{locale}}.ftl", root.display());
        let locales = resolve(&dir, None, &pattern).unwrap();
        assert_eq!(locales.len(), 1);
        assert_eq!(locales[0].locale.to_string(), "de");
        assert_eq!(locales[0].files, [root.join("l10n/de.ftl")]);
    }

    #[test]
    fn resolves_the_module_directory_of_a_module_file() {
        let dir = crate_with(&[
            "src/l10n/en.ftl",
            "src/pages/l10n/de.ftl",
            "src/pages/about/l10n/fr.ftl",
        ]);
        let resolve = |file| resolved(&dir, Some(file), "{mod}/l10n/{locale}.ftl");
        assert_eq!(
            resolve("src/pages/about.rs"),
            expected(&[("fr", &["src/pages/about/l10n/fr.ftl"])])
        );
        assert_eq!(
            resolve("src/pages/mod.rs"),
            expected(&[("de", &["src/pages/l10n/de.ftl"])])
        );
    }

    #[test]
    fn resolves_the_module_directory_of_a_crate_root() {
        let dir = crate_with(&[
            "src/l10n/en.ftl",
            "src/bin/l10n/de.ftl",
            "src/bin/tool/l10n/fr.ftl",
            "tests/l10n/it.ftl",
            "src/pages/main/l10n/es.ftl",
        ]);
        let resolve = |file| resolved(&dir, Some(file), "{mod}/l10n/{locale}.ftl");
        assert_eq!(
            resolve("src/lib.rs"),
            expected(&[("en", &["src/l10n/en.ftl"])])
        );
        assert_eq!(
            resolve("src/main.rs"),
            expected(&[("en", &["src/l10n/en.ftl"])])
        );
        assert_eq!(
            resolve("src/bin/cli.rs"),
            expected(&[("de", &["src/bin/l10n/de.ftl"])])
        );
        assert_eq!(
            resolve("src/bin/tool/main.rs"),
            expected(&[("fr", &["src/bin/tool/l10n/fr.ftl"])])
        );
        assert_eq!(
            resolve("tests/smoke.rs"),
            expected(&[("it", &["tests/l10n/it.ftl"])])
        );
        // A module that happens to be named `main` is not a crate root.
        assert_eq!(
            resolve("src/pages/main.rs"),
            expected(&[("es", &["src/pages/main/l10n/es.ftl"])])
        );
    }

    #[test]
    fn groups_files_below_a_locale_directory() {
        let dir = crate_with(&[
            "l10n/en/main.ftl",
            "l10n/en/forms/login/fields.ftl",
            "l10n/en/notes.md",
            "l10n/de/main.ftl",
            "l10n/README.md",
        ]);
        assert_eq!(
            resolved(&dir, None, "l10n/{locale}/**/*.ftl"),
            expected(&[
                ("de", &["l10n/de/main.ftl"]),
                (
                    "en",
                    &["l10n/en/forms/login/fields.ftl", "l10n/en/main.ftl"]
                ),
            ]),
        );
    }

    #[test]
    fn matches_the_locale_below_recursive_directories() {
        let dir = crate_with(&[
            "l10n/en.ftl",
            "src/a/b/l10n/de.ftl",
            "src/a/l10n-old/fr.ftl",
        ]);
        assert_eq!(
            resolved(&dir, None, "**/l10n/{locale}.ftl"),
            expected(&[("de", &["src/a/b/l10n/de.ftl"]), ("en", &["l10n/en.ftl"])]),
        );
    }

    #[test]
    fn does_not_report_a_file_twice() {
        let dir = crate_with(&["l10n/en/a/main.ftl"]);
        assert_eq!(
            resolved(&dir, None, "l10n/{locale}/**/**/*.ftl"),
            expected(&[("en", &["l10n/en/a/main.ftl"])]),
        );
    }

    #[test]
    fn skips_hidden_entries_unless_the_pattern_names_them() {
        let dir = crate_with(&[
            "l10n/en/main.ftl",
            "l10n/en/.cache/main.ftl",
            "l10n/en/.draft.ftl",
        ]);
        assert_eq!(
            resolved(&dir, None, "l10n/{locale}/**/*.ftl"),
            expected(&[("en", &["l10n/en/main.ftl"])]),
        );
        assert_eq!(
            resolved(&dir, None, "l10n/{locale}/.*.ftl"),
            expected(&[("en", &["l10n/en/.draft.ftl"])]),
        );
    }

    #[test]
    fn skips_directories_at_the_end_of_the_path() {
        let dir = crate_with(&["l10n/en.ftl", "l10n/de.ftl/main.ftl"]);
        assert_eq!(
            resolved(&dir, None, "l10n/{locale}.ftl"),
            expected(&[("en", &["l10n/en.ftl"])]),
        );
    }

    #[test]
    fn rejects_an_invalid_locale() {
        let dir = crate_with(&["l10n/en.ftl", "l10n/e.ftl"]);
        assert!(resolve(&dir, None, "l10n/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_two_spellings_of_one_locale() {
        let dir = crate_with(&["l10n/sl-rozaj-biske.ftl", "l10n/sl-biske-rozaj.ftl"]);
        assert!(resolve(&dir, None, "l10n/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_a_path_without_matches() {
        let dir = crate_with(&["l10n/en.txt"]);
        assert!(resolve(&dir, None, "l10n/{locale}.ftl").is_err());
        assert!(resolve(&dir, None, "missing/{locale}.ftl").is_err());
    }

    #[test]
    fn rejects_source_relative_paths_without_a_source_file() {
        let dir = crate_with(&["l10n/en.ftl", "src/l10n/en.ftl"]);
        assert!(resolve(&dir, None, "./l10n/{locale}.ftl").is_err());
        assert!(resolve(&dir, None, "{mod}/l10n/{locale}.ftl").is_err());
    }

    #[test]
    fn matches_wildcards() {
        assert!(matches_wildcard("*", "main.ftl"));
        assert!(matches_wildcard("*.ftl", "main.ftl"));
        assert!(!matches_wildcard("*.ftl", "main.ftl.bak"));
        assert!(matches_wildcard("a*b*c", "abc"));
        assert!(matches_wildcard("a*b*c", "axxbyyc"));
        assert!(!matches_wildcard("a*b*c", "acb"));
        assert!(!matches_wildcard("a*a", "a"));
        assert!(!matches_wildcard("ab*b", "ab"));
    }
}
