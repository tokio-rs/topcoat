use proc_macro2::LineColumn;

/// A failure encountered while formatting a macro body, carrying the location of
/// the error in the coordinates of the original source file.
#[derive(Debug, Clone)]
pub struct FormatError {
    message: String,
    start: LineColumn,
}

impl FormatError {
    /// Builds a [`FormatError`] from a [`syn::Error`] whose span is measured
    /// relative to a macro body that starts at `base` in the source file.
    ///
    /// A body is parsed as standalone text, so `syn` reports its start as line
    /// 1, column 0 regardless of where it sits in the file. Shifting the
    /// reported position by `base` recovers the location in the file the body
    /// was extracted from.
    #[must_use]
    pub fn new(error: &syn::Error, base: LineColumn) -> Self {
        let local = error.span().start();
        // On the body's first line the column is offset by the column the body
        // starts at; every later line begins at column 0 in both the body's and
        // the file's coordinates, so only the line number shifts.
        let start = if local.line == 1 {
            LineColumn {
                line: base.line,
                column: base.column + local.column,
            }
        } else {
            LineColumn {
                line: base.line + local.line - 1,
                column: local.column,
            }
        };

        Self {
            message: error.to_string(),
            start,
        }
    }

    /// The position of the error in the original source file.
    #[must_use]
    pub fn start(&self) -> LineColumn {
        self.start
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FormatError {}
