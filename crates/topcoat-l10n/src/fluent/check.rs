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

/// Results from checking a compiled app's translations.
///
/// Uses [`RawNamespace`] and [`RawTranslation`] records in the binary to find
/// namespaces and `t!` calls. Reads the corresponding Fluent files from disk, so those
/// files must stay unchanged between the build and this check.
///
/// Reports errors for:
///
/// - messages or attributes used by `t!` but missing from a locale,
/// - message variables that a `t!` call does not supply,
/// - references to missing messages, terms or attributes.
///
/// Reports warnings for:
///
/// - messages unused by both `t!` calls and other messages,
/// - arguments unused by every locale.
#[derive(Debug)]
pub struct Check {
    /// Checked namespaces, sorted by their source locations.
    pub namespaces: Vec<NamespaceCheck>,
}

impl Check {
    /// Checks translations referenced by `binary`.
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
        namespaces.sort_by(|a, b| a.location.cmp(&b.location));
        Self { namespaces }
    }

    /// Checks whether any namespace contains an error.
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

/// Check results for the namespace defined by one `messages!` call.
#[derive(Debug)]
pub struct NamespaceCheck {
    /// The source location of the `messages!` call.
    pub location: Location,
    /// The original Fluent file pattern.
    pub pattern: String,
    /// Translation coverage for each locale.
    pub coverage: Vec<Coverage>,
    /// Reported problems, sorted by severity and then source location.
    pub problems: Vec<Problem>,
}

impl NamespaceCheck {
    /// Checks the namespace's files on disk against its recorded `t!` calls.
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
            location: Location {
                file: PathBuf::from(namespace.source_file),
                line: namespace.line,
                column: Some(namespace.column),
            },
            pattern: namespace.pattern.to_owned(),
            coverage,
            problems,
        }
    }
}

/// The keys a locale provides for a namespace's `t!` calls.
///
/// Keys inherited from fallback locales count as available.
#[derive(Debug, PartialEq, Eq)]
pub struct Coverage {
    /// The locale being checked.
    pub locale: Locale,
    /// The number of distinct keys used by `t!`.
    pub keys: usize,
    /// Missing keys in sorted order, such as `title` or `search.placeholder`.
    pub missing: Vec<String>,
}

/// A translation issue reported by [`Check`].
#[derive(Debug, PartialEq, Eq)]
pub struct Problem {
    /// The issue's severity.
    pub severity: Severity,
    /// The issue's source location.
    pub location: Location,
    /// A one-sentence description of the issue.
    pub message: String,
}

/// The severity assigned to a [`Problem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A problem that produces missing or incorrect text.
    Error,
    /// A likely mistake that does not break the translation.
    Warning,
}

/// A position in Rust source or a Fluent file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// The source file's path.
    pub file: PathBuf,
    /// The line number, counted from 1.
    pub line: u32,
    /// The column number, counted from 1, if available.
    pub column: Option<u32>,
}

/// Formats a source location as `file:line:column`, omitting the column when unknown.
impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file.display(), self.line)?;
        if let Some(column) = self.column {
            write!(f, ":{column}")?;
        }
        Ok(())
    }
}
