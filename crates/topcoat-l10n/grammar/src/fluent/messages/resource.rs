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

/// A Fluent file and its parsed contents.
pub struct FluentFile {
    /// The file's absolute path.
    pub path: PathBuf,
    /// The parsed Fluent entries.
    pub resource: Resource<String>,
}

impl FluentFile {
    /// Loads and parses `path`, attaching errors to `span`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid Fluent syntax.
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

/// Parsed Fluent files belonging to one locale.
pub struct LocaleResources {
    /// The locale shared by these files.
    pub locale: LanguageIdentifier,
    /// Parsed files in input order.
    pub files: Vec<FluentFile>,
}

impl LocaleResources {
    /// Loads and parses a locale's files, attaching errors to `span`.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable files, invalid Fluent syntax or duplicate
    /// message or term definitions.
    pub fn read(files: &LocaleFiles, span: Span) -> syn::Result<Self> {
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
            locale: files.locale.clone(),
            files: parsed,
        };
        match combine(resources.duplicate_errors(span)) {
            Some(err) => Err(err),
            None => Ok(resources),
        }
    }

    /// Reports each repeated message or term definition.
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

/// Creates a syntax error with the file path, line and column in `source`.
fn syntax_error(path: &Path, source: &str, err: &ParserError, span: Span) -> syn::Error {
    let before = source.get(..err.pos.start).unwrap_or(source);
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let line = before.matches('\n').count() + 1;
    let column = before[line_start..].chars().count() + 1;
    syn::Error::new(span, format!("{}:{line}:{column}: {err}", path.display()))
}

/// Merges all errors into one, or returns `None` for an empty input.
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

    /// Creates temporary files and loads them as one locale.
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
        LocaleResources::read(&files, Span::call_site())
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
