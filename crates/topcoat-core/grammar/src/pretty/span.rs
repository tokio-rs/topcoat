use proc_macro2::LineColumn;

/// A source code span representing a range of text from start to end position.
///
/// This type exists because [`proc_macro2::Span`] cannot be constructed from arbitrary
/// line/column positions outside of macro expansion contexts. For pretty printing and
/// testing, we need to create spans from known positions, which `proc_macro2::Span`
/// does not support.
///
/// Unlike [`proc_macro2::Span`], this type stores only position information (start and end
/// [`LineColumn`]) without any hygiene or source file metadata.
///
/// This type is primarily used by the trivia lexer in `trivia.rs` to track the source
/// positions of comments and whitespace during pretty printing.
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Span {
    start: LineColumn,
    end: LineColumn,
}

impl Span {
    /// Creates a new span from start and end positions.
    #[must_use]
    pub fn new(start: LineColumn, end: LineColumn) -> Self {
        Self { start, end }
    }

    /// Returns `true` if this span immediately follows another span with no gap between them.
    ///
    /// Two spans are considered adjacent when this span's start position matches exactly
    /// the other span's end position (same line and column).
    #[must_use]
    pub fn immediately_follows(&self, other: &Span) -> bool {
        self.start.line == other.end.line && self.start.column == other.end.column
    }

    /// Returns `true` if this span comes entirely before the given span.
    ///
    /// This span is considered to come before another if its end position is at or before
    /// the other span's start position.
    #[must_use]
    pub fn comes_before(&self, other: &Span) -> bool {
        self.end.line < other.start.line
            || (self.end.line == other.start.line && self.end.column <= other.start.column)
    }

    /// Returns the starting position of this span.
    #[must_use]
    pub fn start(&self) -> LineColumn {
        self.start
    }

    /// Returns the ending position of this span.
    #[must_use]
    pub fn end(&self) -> LineColumn {
        self.end
    }
}

/// Converts a [`proc_macro2::Span`] to our custom [`Span`] type.
///
/// This conversion extracts only the position information (start and end [`LineColumn`])
/// from the `proc_macro2::Span`, discarding hygiene and source file metadata.
impl From<proc_macro2::Span> for Span {
    fn from(span: proc_macro2::Span) -> Self {
        Span {
            start: span.start(),
            end: span.end(),
        }
    }
}
