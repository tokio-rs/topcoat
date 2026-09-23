use proc_macro2::LineColumn;

/// A range of source text between two [`LineColumn`] positions.
///
/// Contains positions only, without hygiene or source file metadata.
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

    /// Returns whether this span starts exactly where `other` ends.
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
