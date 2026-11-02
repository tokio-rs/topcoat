use std::{
    collections::BTreeSet,
    fs::{self, FileType},
    io,
    path::{Path, PathBuf},
};

use crate::common::{BASE_PLACEHOLDERS, CallSite};

/// A file pattern written in a macro call, such as `"./l10n/{locale}/**/*.ftl"`.
///
/// The pattern is split into components at `/`.
///
/// - `*` matches any characters within one component.
/// - `**` matches any number of directories, including none. It must be a whole
///   component.
/// - An optional capture placeholder, such as `{locale}`, matches a non-empty part of
///   one component and records it. It cannot share a component with `*`.
///
/// The start of the pattern is resolved like other paths in a macro call, so it may
/// begin with a placeholder such as `{mod}`. See [`CallSite::resolve`].
///
/// Wildcards skip names starting with `.` unless the pattern for that name starts
/// with `.` too. `**` does not follow directory symlinks. Other braces and the
/// characters `?`, `[` and `]` are not supported.
#[derive(Clone)]
pub struct Glob {
    /// The leading components without wildcards or placeholders.
    base: String,
    /// The components after `base`.
    components: Vec<GlobComponent>,
}

impl Glob {
    /// Parses `pattern`. With a `capture` name such as `"locale"`, the pattern must
    /// contain exactly one `{locale}` placeholder.
    ///
    /// # Errors
    ///
    /// Returns an error message if the pattern is invalid.
    pub fn parse(pattern: &str, capture: Option<&str>) -> Result<Self, String> {
        let placeholder = capture.map(|name| format!("{{{name}}}"));
        if let Some(placeholder) = &placeholder {
            match pattern.matches(placeholder.as_str()).count() {
                0 => return Err(format!("expected a `{placeholder}` placeholder in the path")),
                1 => {}
                _ => {
                    return Err(format!(
                        "expected exactly one `{placeholder}` placeholder in the path"
                    ));
                }
            }
        }

        let mut base_len = 0;
        let mut components = Vec::new();
        let mut offset = 0;
        for (index, component) in pattern.split('/').enumerate() {
            let parsed = if index == 0 && BASE_PLACEHOLDERS.contains(&component) {
                GlobComponent::Literal(component.to_owned())
            } else {
                GlobComponent::parse(component, placeholder.as_deref())?
            };
            if components.is_empty() && matches!(parsed, GlobComponent::Literal(_)) {
                // Include the `/` after the component, so a pattern such as `/*.txt`
                // keeps its leading `/`.
                base_len = (offset + component.len() + 1).min(pattern.len());
            } else {
                components.push(parsed);
            }
            offset += component.len() + 1;
        }
        Ok(Self {
            base: pattern[..base_len].to_owned(),
            components,
        })
    }

    /// Finds the files matching the pattern, sorted by captured text and then by path.
    ///
    /// # Errors
    ///
    /// Returns an error message if the start of the pattern cannot be resolved or a
    /// directory cannot be read.
    pub fn find(&self, call_site: &CallSite) -> Result<Vec<GlobMatch>, String> {
        let base = call_site.resolve(&self.base)?;
        let mut matches = BTreeSet::new();
        walk(&base, &self.components, None, &mut matches).map_err(|err| err.to_string())?;
        Ok(matches
            .into_iter()
            .map(|(capture, path)| GlobMatch { path, capture })
            .collect())
    }
}

/// A file found by [`Glob::find`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobMatch {
    /// The absolute path of the file.
    pub path: PathBuf,
    /// The text matched by the capture placeholder, if the pattern has one.
    pub capture: Option<String>,
}

/// One part of a [`Glob`], separated from others by `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GlobComponent {
    /// A literal file or directory name.
    Literal(String),
    /// A file or directory pattern containing `*`.
    Wildcard(String),
    /// The `**` wildcard for any number of directories.
    Recursive,
    /// A component containing the capture placeholder, stored as the text on either
    /// side.
    Capture { prefix: String, suffix: String },
}

impl GlobComponent {
    fn parse(component: &str, placeholder: Option<&str>) -> Result<Self, String> {
        let unexpected_brace = || match placeholder {
            Some(placeholder) => {
                format!("unexpected brace in the path; only `{placeholder}` is supported")
            }
            None => "unexpected brace in the path".to_owned(),
        };
        if component.contains(['?', '[', ']']) {
            return Err("`?`, `[` and `]` are not supported in the path".to_owned());
        }
        if component == "**" {
            return Ok(Self::Recursive);
        }
        if component.contains("**") {
            return Err("`**` must be a whole path component".to_owned());
        }
        if let Some(placeholder) = placeholder
            && let Some((prefix, suffix)) = component.split_once(placeholder)
        {
            if prefix.contains('*') || suffix.contains('*') {
                return Err(format!(
                    "`*` cannot appear in the same path component as `{placeholder}`"
                ));
            }
            if prefix.contains(['{', '}']) || suffix.contains(['{', '}']) {
                return Err(unexpected_brace());
            }
            return Ok(Self::Capture {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
            });
        }
        if component.contains(['{', '}']) {
            return Err(unexpected_brace());
        }
        if component.contains('*') {
            Ok(Self::Wildcard(component.to_owned()))
        } else {
            Ok(Self::Literal(component.to_owned()))
        }
    }
}

/// Matched files with the text captured by the placeholder.
type Matches = BTreeSet<(Option<String>, PathBuf)>;

/// Searches `dir` for paths matching `components` and adds matching files to `matches`.
/// The `capture` argument holds the text captured so far.
fn walk(
    dir: &Path,
    components: &[GlobComponent],
    capture: Option<&str>,
    matches: &mut Matches,
) -> io::Result<()> {
    let Some((component, rest)) = components.split_first() else {
        if dir.is_file() {
            matches.insert((capture.map(str::to_owned), dir.to_owned()));
        }
        return Ok(());
    };

    match component {
        GlobComponent::Literal(name) if name.is_empty() || name == "." => {
            walk(dir, rest, capture, matches)
        }
        GlobComponent::Literal(name) if name == ".." => {
            walk(dir.parent().unwrap_or(dir), rest, capture, matches)
        }
        GlobComponent::Literal(name) => walk(&dir.join(name), rest, capture, matches),
        GlobComponent::Wildcard(pattern) => {
            for (name, _) in entries(dir)? {
                if is_visible(&name, pattern) && matches_wildcard(pattern, &name) {
                    walk(&dir.join(name), rest, capture, matches)?;
                }
            }
            Ok(())
        }
        GlobComponent::Recursive => {
            walk(dir, rest, capture, matches)?;
            for (name, file_type) in entries(dir)? {
                if file_type.is_dir() && is_visible(&name, "") {
                    walk(&dir.join(name), components, capture, matches)?;
                }
            }
            Ok(())
        }
        GlobComponent::Capture { prefix, suffix } => {
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

/// Lists directory entries with their names and types, sorted by name. Skips names that
/// are not valid UTF-8 and returns an empty list if `dir` is not a directory.
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

/// Tests a wildcard against `name`. A name starting with `.` requires a pattern
/// starting with `.`.
fn is_visible(name: &str, pattern: &str) -> bool {
    !name.starts_with('.') || pattern.starts_with('.')
}

/// Tests `name` against a pattern where `*` matches any sequence of characters.
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

    fn parse(pattern: &str) -> Result<Glob, String> {
        Glob::parse(pattern, Some("name"))
    }

    fn literal(name: &str) -> GlobComponent {
        GlobComponent::Literal(name.to_owned())
    }

    fn capture(prefix: &str, suffix: &str) -> GlobComponent {
        GlobComponent::Capture {
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
        }
    }

    #[test]
    fn splits_off_the_literal_start() {
        let glob = parse("./data/items-{name}.txt").unwrap();
        assert_eq!(glob.base, "./data/");
        assert_eq!(glob.components, [capture("items-", ".txt")]);
    }

    #[test]
    fn keeps_a_leading_slash_in_the_start() {
        let glob = parse("/{name}.txt").unwrap();
        assert_eq!(glob.base, "/");
    }

    #[test]
    fn keeps_wildcards_after_the_first_one() {
        let glob = parse("./**/data/{name}/**/*.txt").unwrap();
        assert_eq!(glob.base, "./");
        assert_eq!(
            glob.components,
            [
                GlobComponent::Recursive,
                literal("data"),
                capture("", ""),
                GlobComponent::Recursive,
                GlobComponent::Wildcard("*.txt".to_owned()),
            ]
        );
    }

    #[test]
    fn accepts_a_leading_module_directory() {
        let glob = parse("{mod}/data/{name}.txt").unwrap();
        assert_eq!(glob.base, "{mod}/data/");
    }

    #[test]
    fn rejects_a_module_directory_after_the_start() {
        assert!(parse("./{mod}/{name}.txt").is_err());
        assert!(parse("data/{mod}/{name}.txt").is_err());
        assert!(parse("{mod}-x/{name}.txt").is_err());
    }

    #[test]
    fn requires_exactly_one_placeholder() {
        assert!(parse("./data/a.txt").is_err());
        assert!(parse("./data/{name}/{name}.txt").is_err());
        assert!(parse("./data/{name}{name}.txt").is_err());
    }

    #[test]
    fn rejects_placeholders_without_a_capture() {
        assert!(Glob::parse("./data/*.txt", None).is_ok());
        assert!(Glob::parse("./data/{name}.txt", None).is_err());
    }

    #[test]
    fn rejects_other_braces() {
        assert!(parse("./data/{name}/{a,b}.txt").is_err());
        assert!(parse("./data/{other}/{name}.txt").is_err());
        assert!(parse("./data/{name}}.txt").is_err());
    }

    #[test]
    fn rejects_unsupported_glob_syntax() {
        assert!(parse("./data/{name}/v?.txt").is_err());
        assert!(parse("./data/{name}/[ab].txt").is_err());
        assert!(parse("./data/{name}/**.txt").is_err());
        assert!(parse("./a**/{name}.txt").is_err());
    }

    #[test]
    fn rejects_a_wildcard_next_to_the_placeholder() {
        assert!(parse("./data/{name}-*.txt").is_err());
        assert!(parse("./data/*{name}/main.txt").is_err());
    }

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

    /// Lists the files found by `pattern` from `src/lib.rs`, with their captures and
    /// relative to the test crate.
    fn found(dir: &TempDir, capture: Option<&str>, pattern: &str) -> Vec<(Option<String>, String)> {
        let root = std::path::absolute(dir.path()).unwrap();
        let call_site = CallSite::new(&root, Some(&root.join("src/lib.rs")), None).unwrap();
        Glob::parse(pattern, capture)
            .unwrap()
            .find(&call_site)
            .unwrap()
            .into_iter()
            .map(|found| {
                let relative = found.path.strip_prefix(&root).unwrap();
                let relative = relative.to_str().unwrap().replace('\\', "/");
                (found.capture, relative)
            })
            .collect()
    }

    fn expected(files: &[(&str, &str)]) -> Vec<(Option<String>, String)> {
        files
            .iter()
            .map(|(capture, file)| (Some((*capture).to_owned()), (*file).to_owned()))
            .collect()
    }

    #[test]
    fn resolves_the_start_from_the_call_site() {
        let dir = crate_with(&["data/a.txt", "src/data/b.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "data/{name}.txt"),
            expected(&[("a", "data/a.txt")])
        );
        assert_eq!(
            found(&dir, Some("name"), "./data/{name}.txt"),
            expected(&[("b", "src/data/b.txt")])
        );
    }

    #[test]
    fn sorts_by_capture_and_then_by_path() {
        let dir = crate_with(&["data/b/2.txt", "data/b/1.txt", "data/a/3.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "data/{name}/*.txt"),
            expected(&[("a", "data/a/3.txt"), ("b", "data/b/1.txt"), ("b", "data/b/2.txt")])
        );
    }

    #[test]
    fn finds_files_without_a_capture() {
        let dir = crate_with(&["data/a.txt", "data/b.md"]);
        assert_eq!(
            found(&dir, None, "data/*.txt"),
            [(None, "data/a.txt".to_owned())]
        );
    }

    #[test]
    fn matches_any_number_of_directories() {
        let dir = crate_with(&["data/x.txt", "src/a/b/data/y.txt", "src/a/data-old/z.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "**/data/{name}.txt"),
            expected(&[("x", "data/x.txt"), ("y", "src/a/b/data/y.txt")])
        );
    }

    #[test]
    fn does_not_report_a_file_twice() {
        let dir = crate_with(&["data/a/b/main.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "data/{name}/**/**/*.txt"),
            expected(&[("a", "data/a/b/main.txt")])
        );
    }

    #[test]
    fn skips_hidden_entries_unless_the_pattern_names_them() {
        let dir = crate_with(&["data/a/main.txt", "data/a/.cache/main.txt", "data/a/.draft.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "data/{name}/**/*.txt"),
            expected(&[("a", "data/a/main.txt")])
        );
        assert_eq!(
            found(&dir, Some("name"), "data/{name}/.*.txt"),
            expected(&[("a", "data/a/.draft.txt")])
        );
    }

    #[test]
    fn skips_directories_at_the_end_of_the_pattern() {
        let dir = crate_with(&["data/a.txt", "data/b.txt/main.txt"]);
        assert_eq!(
            found(&dir, Some("name"), "data/{name}.txt"),
            expected(&[("a", "data/a.txt")])
        );
    }

    #[test]
    fn matches_wildcards() {
        assert!(matches_wildcard("*", "main.txt"));
        assert!(matches_wildcard("*.txt", "main.txt"));
        assert!(!matches_wildcard("*.txt", "main.txt.bak"));
        assert!(matches_wildcard("a*b*c", "abc"));
        assert!(matches_wildcard("a*b*c", "axxbyyc"));
        assert!(!matches_wildcard("a*b*c", "acb"));
        assert!(!matches_wildcard("a*a", "a"));
        assert!(!matches_wildcard("ab*b", "ab"));
    }
}
