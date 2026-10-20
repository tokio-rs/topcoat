use std::{
    collections::{HashMap, hash_map::Entry as MapEntry},
    fs,
    path::{Path, PathBuf},
};

use fluent_syntax::{
    ast::{Entry, Resource},
    parser::{self, ParserError},
};
use icu_locale_core::LanguageIdentifier;
use proc_macro2::Span;

use crate::fluent::LocaleFiles;

/// A parsed Fluent file.
pub struct FluentFile {
    /// The absolute path of the file.
    pub path: PathBuf,
    /// The entries of the file.
    pub resource: Resource<String>,
}

impl FluentFile {
    /// Reads and parses the Fluent file at `path`, reporting errors at `span`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or has syntax errors.
    pub fn read(path: &Path, span: Span) -> syn::Result<Self> {
        let source = fs::read_to_string(path).map_err(|err| {
            syn::Error::new(span, format!("failed to read `{}`: {err}", path.display()))
        })?;
        let (resource, errors) = match parser::parse(source.clone()) {
            Ok(resource) => (resource, Vec::new()),
            Err(failure) => failure,
        };
        let errors = errors
            .iter()
            .map(|err| syntax_error(path, &source, err, span));
        match combine(errors) {
            Some(err) => Err(err),
            None => Ok(Self {
                path: path.to_owned(),
                resource,
            }),
        }
    }
}

/// The parsed Fluent files of one locale.
pub struct LocaleResources {
    /// The locale of the files.
    pub locale: LanguageIdentifier,
    /// The files, in the order they were given.
    pub files: Vec<FluentFile>,
}

impl LocaleResources {
    /// Reads and parses the files of a locale, reporting errors at `span`.
    ///
    /// # Errors
    ///
    /// Fails when a file cannot be read or has syntax errors, or when a message
    /// or term is defined more than once across the files.
    pub fn read(files: LocaleFiles, span: Span) -> syn::Result<Self> {
        let mut parsed = Vec::with_capacity(files.files.len());
        let mut errors = Vec::new();
        for path in &files.files {
            match FluentFile::read(path, span) {
                Ok(file) => parsed.push(file),
                Err(err) => errors.push(err),
            }
        }
        if let Some(err) = combine(errors) {
            return Err(err);
        }

        let resources = Self {
            locale: files.locale,
            files: parsed,
        };
        match combine(resources.duplicate_errors(span)) {
            Some(err) => Err(err),
            None => Ok(resources),
        }
    }

    /// Returns an error for each message or term defined after its first
    /// definition.
    fn duplicate_errors(&self, span: Span) -> Vec<syn::Error> {
        let mut first = HashMap::new();
        let mut errors = Vec::new();
        for file in &self.files {
            for entry in &file.resource.body {
                let id = match entry {
                    Entry::Message(message) => format!("`{}`", message.id.name),
                    Entry::Term(term) => format!("`-{}`", term.id.name),
                    _ => continue,
                };
                match first.entry(id) {
                    MapEntry::Vacant(entry) => {
                        entry.insert(&file.path);
                    }
                    MapEntry::Occupied(entry) => {
                        let (id, path) = (entry.key(), entry.get());
                        let message = if **path == file.path {
                            format!("{id} is defined twice in `{}`", file.path.display())
                        } else {
                            format!(
                                "{id} is defined in both `{}` and `{}`",
                                path.display(),
                                file.path.display()
                            )
                        };
                        errors.push(syn::Error::new(span, message));
                    }
                }
            }
        }
        errors
    }
}

/// Returns an error for a syntax error in `source`, the contents of `path`,
/// naming the line and column where it occurs.
fn syntax_error(path: &Path, source: &str, err: &ParserError, span: Span) -> syn::Error {
    let before = source.get(..err.pos.start).unwrap_or(source);
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let line = before.matches('\n').count() + 1;
    let column = before[line_start..].chars().count() + 1;
    syn::Error::new(span, format!("{}:{line}:{column}: {err}", path.display()))
}

/// Combines `errors` into one error that reports each of them, or returns
/// `None` when there are none.
fn combine(errors: impl IntoIterator<Item = syn::Error>) -> Option<syn::Error> {
    errors.into_iter().reduce(|mut combined, err| {
        combined.combine(err);
        combined
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Writes the files to a temporary directory and reads them as the files of
    /// one locale.
    fn read(files: &[(&str, &str)]) -> syn::Result<LocaleResources> {
        let dir = TempDir::new().unwrap();
        let mut paths = Vec::new();
        for (name, source) in files {
            let path = dir.path().join(name);
            fs::write(&path, source).unwrap();
            paths.push(path);
        }
        let files = LocaleFiles {
            locale: LanguageIdentifier::UNKNOWN,
            files: paths,
        };
        LocaleResources::read(files, Span::call_site())
    }

    fn message_ids(resources: &LocaleResources) -> Vec<String> {
        resources
            .files
            .iter()
            .flat_map(|file| &file.resource.body)
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reads_the_messages_of_every_file() {
        let resources = read(&[
            ("a.ftl", "hello = Hello\n"),
            ("b.ftl", "-brand = Topcoat\nbye = Bye from { -brand }\n"),
        ])
        .unwrap();
        assert_eq!(message_ids(&resources), ["hello", "bye"]);
    }

    #[test]
    fn reports_the_location_of_a_syntax_error() {
        let err = read(&[("a.ftl", "hello = Hello\nbroken = { $ }\nbye = Bye\n")])
            .err()
            .unwrap();
        assert!(err.to_string().contains("a.ftl:2:"), "{err}");
    }

    #[test]
    fn reports_syntax_errors_of_every_file() {
        let err = read(&[("a.ftl", "a = {\n"), ("b.ftl", "b = {\n")])
            .err()
            .unwrap();
        let messages: Vec<_> = err.into_iter().map(|err| err.to_string()).collect();
        assert!(messages.iter().any(|message| message.contains("a.ftl")));
        assert!(messages.iter().any(|message| message.contains("b.ftl")));
    }

    #[test]
    fn rejects_a_message_defined_twice() {
        assert!(read(&[("a.ftl", "hello = Hello\nhello = Hi\n")]).is_err());
        assert!(read(&[("a.ftl", "hello = Hello\n"), ("b.ftl", "hello = Hi\n")]).is_err());
    }

    #[test]
    fn rejects_a_term_defined_twice() {
        assert!(read(&[("a.ftl", "-brand = A\n"), ("b.ftl", "-brand = B\n")]).is_err());
    }

    #[test]
    fn accepts_a_message_and_term_of_the_same_name() {
        assert!(read(&[("a.ftl", "brand = A\n-brand = B\n")]).is_ok());
    }
}
