use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Parens, braces, brackets
    LAngle,   // <
    RAngle,   // >
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    LParen,   // (
    RParen,   // )

    Ident,
    Literal,

    Eq, // =

    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    kind: TokenKind,
    text: Cow<'a, str>,
    span: Span,
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind, text: impl Into<Cow<'a, str>>, span: Span) -> Self {
        Self {
            kind,
            text: text.into(),
            span,
        }
    }
}
