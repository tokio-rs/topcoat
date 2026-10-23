mod analysis;
mod sources;
mod syntax;

use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    path::PathBuf,
};

use analysis::analyze;
use sources::Sources;
use syntax::{ParsedFile, visit_expressions};

use crate::{Locale, RawNamespace, RawTranslation};

/// The problems with the messages of a compiled binary.
///
/// The binary names its namespaces and `t!` calls in records, which
/// [`RawNamespace`] and [`RawTranslation`] describe. The check reads the
/// Fluent files of each namespace from disk and compares them with the `t!`
/// calls that use the namespace, so the files must not change between the
/// build and the check.
///
/// It reports these errors:
///
/// - a message or attribute that a `t!` call uses, but a locale lacks,
/// - a variable of a message that a `t!` call does not pass,
/// - a reference to a message, term or attribute that does not exist.
///
/// It reports these warnings:
///
/// - a message that no `t!` call or other message uses,
/// - an argument that no locale uses.
#[derive(Debug)]
pub struct Check {
    /// The namespaces of the binary, sorted by where they are invoked.
    pub namespaces: Vec<NamespaceCheck>,
}

impl Check {
    /// Checks the messages of the compiled `binary`.
    #[must_use]
    pub fn binary(binary: &[u8]) -> Self {
        let mut translations: BTreeMap<&str, Vec<RawTranslation<'_>>> = BTreeMap::new();
        for translation in RawTranslation::find_in_binary(binary) {
            translations
                .entry(translation.namespace)
                .or_default()
                .push(translation);
        }
        let mut namespaces: Vec<_> = RawNamespace::find_in_binary(binary)
            .iter()
            .map(|namespace| {
                let calls = translations.remove(namespace.id).unwrap_or_default();
                NamespaceCheck::new(namespace, calls)
            })
            .collect();
        namespaces.sort_by(|a, b| (&a.source_file, a.line).cmp(&(&b.source_file, b.line)));
        Self { namespaces }
    }

    /// Returns whether any namespace has an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.namespaces.iter().any(|namespace| {
            namespace
                .problems
                .iter()
                .any(|problem| problem.severity == Severity::Error)
        })
    }
}

/// The result of checking one namespace, created by one `messages!`.
#[derive(Debug)]
pub struct NamespaceCheck {
    /// The source file that invokes `messages!`.
    pub source_file: PathBuf,
    /// The line of the invocation, starting at 1.
    pub line: u32,
    /// The path of the Fluent files, as written.
    pub pattern: String,
    /// How many of the keys that `t!` calls use each locale has, by locale.
    pub coverage: Vec<Coverage>,
    /// The problems, errors first, then sorted by location.
    pub problems: Vec<Problem>,
}

impl NamespaceCheck {
    /// Checks the current Fluent files of `namespace` against `calls`, its
    /// `t!` calls.
    fn new(namespace: &RawNamespace<'_>, mut calls: Vec<RawTranslation<'_>>) -> Self {
        // A call can be found more than once, such as when the linker keeps two
        // copies of its record.
        calls.sort_by(|a, b| (a.file, a.line, a.column).cmp(&(b.file, b.line, b.column)));
        calls.dedup();
        let mut problems = Vec::new();
        let sources = Sources::load(namespace, &mut problems);
        let coverage = analyze(&sources, &calls, &mut problems);
        problems.sort_by(|a, b| (a.severity, &a.location).cmp(&(b.severity, &b.location)));
        Self {
            source_file: PathBuf::from(namespace.source_file),
            line: namespace.line,
            pattern: namespace.pattern.to_owned(),
            coverage,
            problems,
        }
    }
}

/// How many of the keys that the `t!` calls of a namespace use one locale has.
///
/// A locale has a key when its own files or the files of a locale it takes
/// messages from define it.
#[derive(Debug, PartialEq, Eq)]
pub struct Coverage {
    /// The locale.
    pub locale: Locale,
    /// The number of keys that `t!` calls use.
    pub keys: usize,
    /// The keys the locale lacks, sorted, such as `title` or
    /// `search.placeholder`.
    pub missing: Vec<String>,
}

/// A problem found by a [`Check`].
#[derive(Debug, PartialEq, Eq)]
pub struct Problem {
    /// Whether the problem is an error or a warning.
    pub severity: Severity,
    /// Where the problem is.
    pub location: Location,
    /// What is wrong, in one sentence.
    pub message: String,
}

/// Whether a [`Problem`] is an error or a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something that shows a placeholder or wrong text.
    Error,
    /// Something that is likely a mistake, but does no harm.
    Warning,
}

/// A place in a source file or a Fluent file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// The path of the file.
    pub file: PathBuf,
    /// The line, starting at 1.
    pub line: u32,
    /// The column, starting at 1, when known.
    pub column: Option<u32>,
}

/// Shows the location as `file:line:column`, or `file:line` without a column.
impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file.display(), self.line)?;
        if let Some(column) = self.column {
            write!(f, ":{column}")?;
        }
        Ok(())
    }
}
