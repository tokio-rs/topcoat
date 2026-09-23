use proc_macro2::LineColumn;

/// A range of source text, from a start position to an end position.
///
/// Unlike [`proc_macro2::Span`], a `Span` can be created from any pair of
/// line and column positions. It holds only the two positions.
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

    /// Returns whether this span ends at or before the start of `other`.
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

/// Keeps only the start and end positions of a [`proc_macro2::Span`].
impl From<proc_macro2::Span> for Span {
    fn from(span: proc_macro2::Span) -> Self {
        Span {
            start: span.start(),
            end: span.end(),
        }
    }
}
