use proc_macro2::LineColumn;

/// An error found while formatting a macro body, with its position in the
/// source file.
#[derive(Debug, Clone)]
pub struct FormatError {
    message: String,
    start: LineColumn,
}

impl FormatError {
    /// Builds a [`FormatError`] from a [`syn::Error`] whose span is relative
    /// to a macro body that starts at `base` in the source file.
    ///
    /// A body is parsed on its own, so `syn` reports positions as if the body
    /// started at line 1, column 0. This shifts the position by `base` to get
    /// the position in the file.
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

    /// Returns the position of the error in the source file.
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
