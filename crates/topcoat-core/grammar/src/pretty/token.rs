use std::borrow::Cow;

use crate::pretty::RingBuffer;

/// Whether a piece of text prints, depending on whether the enclosing group
/// breaks.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TextMode {
    /// The text always prints.
    Always,
    /// The text prints only when the enclosing group fits on one line.
    NoBreak,
    /// The text prints only when the enclosing group breaks across lines,
    /// like a trailing comma.
    Break,
}

/// How the breaks in a group behave when the group does not fit on one line.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BreakMode {
    /// Every break in the group becomes a line break, as in a block body.
    Consistent,
    /// Each break becomes a line break only when the text after it does not
    /// fit, which fills lines like word wrapping.
    Inconsistent,
}

#[derive(Debug)]
pub(super) enum Token<'a> {
    Text(TextToken<'a>),
    Break(BreakToken),
    ForceBreak,
    Begin(BeginToken),
    End,
}

#[derive(Debug)]
pub(super) struct TextToken<'a> {
    string: Cow<'a, str>,
    mode: TextMode,
}

impl<'a> TextToken<'a> {
    pub(super) fn new(string: Cow<'a, str>, mode: TextMode) -> Self {
        Self { string, mode }
    }

    pub(super) fn string(&self) -> &str {
        &self.string
    }

    pub(super) fn mode(&self) -> TextMode {
        self.mode
    }
}

#[derive(Debug)]
pub(super) struct BreakToken {
    len: isize,
    indent: isize,
}

impl BreakToken {
    pub(super) fn new(len: isize, indent: isize) -> Self {
        Self { len, indent }
    }

    pub fn push_len(&mut self, len: isize) {
        self.len += len;
    }

    pub(super) fn len(&self) -> isize {
        self.len
    }

    pub(super) fn indent(&self) -> isize {
        self.indent
    }
}

#[derive(Debug)]
pub(super) struct BeginToken {
    mode: BreakMode,
    len: isize,
}

impl BeginToken {
    pub fn push_len(&mut self, len: isize) {
        self.len += len;
    }

    pub(super) fn mode(&self) -> BreakMode {
        self.mode
    }

    pub fn len(&self) -> isize {
        self.len
    }
}

impl BeginToken {
    pub(super) fn new(mode: BreakMode, len: isize) -> Self {
        Self { mode, len }
    }
}

pub(super) struct TokenBuffer<'a> {
    tokens: RingBuffer<Token<'a>>,
    last_break: Option<usize>,
    begin_stack: Vec<usize>,
}

impl<'a> TokenBuffer<'a> {
    pub fn new() -> Self {
        Self {
            tokens: RingBuffer::new(),
            last_break: None,
            begin_stack: Vec::new(),
        }
    }

    pub fn last_break_mut(&mut self) -> Option<&mut BreakToken> {
        match &self.last_break {
            Some(index) => match &mut self.tokens[*index] {
                Token::Break(break_token) => Some(break_token),
                _ => unreachable!(),
            },
            _ => None,
        }
    }

    pub fn current_begin_mut(&mut self) -> Option<&mut BeginToken> {
        match self.begin_stack.last() {
            Some(index) => match &mut self.tokens[*index] {
                Token::Begin(begin_token) => Some(begin_token),
                _ => unreachable!(),
            },
            _ => None,
        }
    }

    pub fn push_len(&mut self, len: isize) {
        if let Some(last_break) = self.last_break_mut() {
            last_break.push_len(len);
        }
        if let Some(current_begin) = self.current_begin_mut() {
            current_begin.push_len(len);
        }
    }

    pub fn push_back(&mut self, token: Token<'a>) {
        match &token {
            Token::Text(_) | Token::ForceBreak => {}
            Token::Break(_) => self.last_break = Some(self.tokens.next_index()),
            Token::Begin(_) => self.begin_stack.push(self.tokens.next_index()),
            Token::End => {
                self.begin_stack.pop();
                self.last_break = None;
            }
        }
        self.tokens.push_back(token);
    }

    pub fn pop_front(&mut self) -> Option<Token<'a>> {
        self.tokens.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
