use std::{fs, path::PathBuf};

use super::{Location, Problem, Severity};
use crate::{Locale, RawNamespace};

/// A namespace's Fluent source files loaded from disk.
pub(super) struct Sources {
    /// File paths and contents grouped by locale.
    pub(super) locales: Vec<(Locale, Vec<(PathBuf, String)>)>,
}

impl Sources {
    /// Loads the files recorded for `namespace` and reports each read failure in
    /// `problems`.
    pub(super) fn load(namespace: &RawNamespace<'_>, problems: &mut Vec<Problem>) -> Self {
        let locales = namespace
            .locales
            .iter()
            .filter_map(|locale| {
                let files = locale.files.iter().filter_map(|path| {
                    let path = PathBuf::from(path);
                    match fs::read_to_string(&path) {
                        Ok(text) => Some((path, text)),
                        Err(err) => {
                            problems.push(Problem {
                                severity: Severity::Error,
                                message: format!("failed to read the file: {err}"),
                                location: Location {
                                    file: path,
                                    line: 1,
                                    column: None,
                                },
                            });
                            None
                        }
                    }
                });
                let files = files.collect();
                Some((Locale::try_from_str(locale.locale).ok()?, files))
            })
            .collect();
        Self { locales }
    }
}
