use std::borrow::Cow;

use proc_macro2::LineColumn;

use crate::pretty::{
    BeginToken, BreakMode, BreakToken, TextMode, TextToken, Token, TokenBuffer, Trivia, TriviaKind,
    registry::Registry,
};

/// The target line width. Groups whose collapsed length exceeds this break.
pub const MARGIN: isize = 89;
/// Number of spaces added per indent level.
pub const INDENT: isize = 4;
/// Floor on the printer's available space so that deeply nested content keeps
/// breaking onto its own lines instead of running off the right margin.
pub const MIN_SPACE: isize = 60;

#[derive(Debug)]
struct PrintFrame {
    group_break: bool,
}

/// The pretty-printing engine. Implements a Wadler/Oppen-style two-pass
/// algorithm: callers feed in a stream of tokens (text, breaks, group
/// boundaries) via the `scan_*` methods, and the printer decides which breaks
/// to render based on the available width.
pub struct Printer<'a> {
    registry: &'a Registry,
    trivia: &'a [Trivia<'a>],
    tokens: TokenBuffer<'a>,
    output: String,
    space: isize,
    scan_indent: isize,
    print_indent: isize,
    print_frames: Vec<PrintFrame>,
    pending_break: bool,
    cursor: LineColumn,
}

impl<'a> Printer<'a> {
    #[must_use]
    pub fn new(
        registry: &'a Registry,
        trivia: &'a [Trivia<'a>],
        initial_space: isize,
        initial_indent: isize,
    ) -> Self {
        Self {
            registry,
            trivia,
            output: String::new(),
            space: initial_space.max(MIN_SPACE),
            scan_indent: initial_indent,
            print_indent: 0,
            tokens: TokenBuffer::new(),
            print_frames: Vec::new(),
            pending_break: false,
            cursor: LineColumn { line: 1, column: 0 },
        }
    }

    #[must_use]
    pub fn registry(&self) -> &'a Registry {
        self.registry
    }

    #[must_use]
    pub fn cursor(&self) -> LineColumn {
        self.cursor
    }

    pub fn move_cursor(&mut self, cursor: LineColumn) {
        self.cursor = cursor;
    }

    pub fn advance_cursor(&mut self, string: &str) {
        for char in string.chars() {
            match char {
                '\n' => {
                    self.cursor.line += 1;
                    self.cursor.column = 0;
                }
                _ => self.cursor.column += 1,
            }
        }
    }

    /// Pushes a text token onto the buffer.
    ///
    /// # Panics
    ///
    /// Panics if `string.len()` does not fit in an `isize`.
    pub fn scan_text(&mut self, string: Cow<'static, str>, mode: TextMode) {
        self.tokens.push_len(string.len().try_into().unwrap());
        let token = Token::Text(TextToken::new(string, mode));
        self.tokens.push_back(token);
    }

    pub fn scan_break(&mut self) {
        self.tokens
            .push_back(Token::Break(BreakToken::new(0, self.scan_indent)));
    }

    pub fn scan_force_break(&mut self) {
        let len = MARGIN;
        self.tokens.push_back(Token::ForceBreak);
        self.tokens.push_len(len);
    }

    pub fn scan_indent(&mut self, indent: isize) {
        self.scan_indent += indent;
    }

    #[must_use]
    pub fn current_indent(&self) -> isize {
        self.scan_indent
    }

    pub fn scan_begin(&mut self, mode: BreakMode) {
        self.tokens
            .push_back(Token::Begin(BeginToken::new(mode, 0)));
    }

    /// # Panics
    ///
    /// Panics if there was no matching call to [`scan_begin`](Self::scan_begin) prior to running
    /// this function.
    pub fn scan_end(&mut self) {
        let len = self
            .tokens
            .current_begin_mut()
            .expect("scanned end without matching begin")
            .len();
        self.tokens.push_back(Token::End);
        // Add child block length to parent.
        if let Some(parent) = self.tokens.current_begin_mut() {
            parent.push_len(len);
        }
    }

    pub fn scan_no_break_trivia(&mut self) {
        while let Some(trivia) = self.ready_trivia() {
            match trivia.kind {
                TriviaKind::BlockComment => {
                    self.scan_text(" ".into(), TextMode::Always);
                    self.scan_text(trivia.content.to_string().into(), TextMode::Always);
                    self.pop_trivia();
                }
                TriviaKind::LineComment => {
                    // Line comments are banned in no-break areas.
                    break;
                }
                TriviaKind::Whitespace => {
                    self.pop_trivia();
                }
            }
        }
    }

    pub fn scan_same_line_trivia(&mut self) {
        while let Some(trivia) = self.ready_trivia() {
            match trivia.kind {
                TriviaKind::BlockComment => {
                    self.scan_text(" ".into(), TextMode::Always);
                    self.scan_text(trivia.content.to_string().into(), TextMode::Always);
                    self.pop_trivia();
                    break;
                }
                TriviaKind::LineComment => {
                    self.scan_text(" ".into(), TextMode::Always);
                    self.scan_text(trivia.content.to_string().into(), TextMode::Always);
                    self.scan_force_break();
                    self.pop_trivia();
                    break;
                }
                TriviaKind::Whitespace => {
                    if trivia.newlines() > 0 {
                        break;
                    }
                    self.pop_trivia();
                }
            }
        }
    }

    pub fn scan_trivia(&mut self, leading_whitespace: bool, trailing_whitespace: bool) {
        // let break_mode = self.tokens.current_begin_mut().unwrap().mode();
        let mut encountered_comment = false;
        let mut pending_newlines = 0;
        while let Some(trivia) = self.ready_trivia() {
            match trivia.kind {
                TriviaKind::BlockComment => {
                    for _ in 0..pending_newlines {
                        self.scan_break();
                        self.scan_text(" ".into(), TextMode::Always);
                    }
                    if leading_whitespace || encountered_comment {
                        self.scan_text(" ".into(), TextMode::Always);
                    }
                    self.scan_text(trivia.content.to_string().into(), TextMode::Always);
                    pending_newlines = 1;
                    self.pop_trivia();
                    encountered_comment = true;
                }
                TriviaKind::LineComment => {
                    for _ in 0..pending_newlines {
                        self.scan_break();
                        self.scan_text(" ".into(), TextMode::Always);
                    }
                    if leading_whitespace || encountered_comment {
                        self.scan_text(" ".into(), TextMode::Always);
                    }
                    self.scan_text(trivia.content.to_string().into(), TextMode::Always);
                    self.scan_force_break();
                    pending_newlines = 1;
                    self.pop_trivia();
                    encountered_comment = true;
                }
                TriviaKind::Whitespace => {
                    if (leading_whitespace || encountered_comment) && trivia.newlines() > 1 {
                        pending_newlines += 1;
                    }
                    self.pop_trivia();
                }
            }
        }
        if trailing_whitespace {
            for _ in 0..pending_newlines {
                self.scan_break();
                self.scan_text(" ".into(), TextMode::Always);
            }
        }
    }

    pub fn skip_trivia(&mut self) {
        while self.ready_trivia().is_some() {
            self.trivia = &self.trivia[1..];
        }
    }

    /// Whether an as-yet-unemitted comment (line or block) begins before the
    /// given source position. Lets an otherwise-empty construct tell an interior
    /// comment apart from plain whitespace and decide whether it still needs to
    /// break its content onto separate lines.
    #[must_use]
    pub fn has_comment_before(&self, pos: LineColumn) -> bool {
        for trivia in self.trivia {
            if trivia.span.start() >= pos {
                break;
            }
            if !matches!(trivia.kind, TriviaKind::Whitespace) {
                return true;
            }
        }
        false
    }

    fn ready_trivia(&mut self) -> Option<&'a Trivia<'a>> {
        if let Some(trivia) = self.trivia.first()
            && trivia.span.start() <= self.cursor
        {
            return Some(trivia);
        }
        None
    }

    fn pop_trivia(&mut self) {
        if self.cursor < self.trivia[0].span.end() {
            self.move_cursor(self.trivia[0].span.end());
        }
        self.trivia = &self.trivia[1..];
    }

    fn line_dirty(&self) -> bool {
        if let Some(last) = self.output.chars().last() {
            return last != '\n';
        }
        true
    }

    fn print_string(&mut self, mut string: &str) {
        if self.pending_break {
            self.print_break();
        }
        if !self.line_dirty() {
            string = string.trim_start();
        } else if self.output.ends_with(' ') {
            // Collapse redundant soft spaces. A break that stays flat renders as
            // nothing but leaves its accompanying space behind, so two adjacent
            // soft breaks (e.g. a trailing comment's break and the closing
            // delimiter's) would otherwise produce a doubled space.
            string = string.trim_start_matches(' ');
        }
        if string.is_empty() {
            return;
        }
        self.print_indent();
        self.output.push_str(string);
        self.space -= isize::try_from(string.len()).unwrap();
    }

    fn print_break(&mut self) {
        self.output.push('\n');
        // Subtract the new line's indentation from the available space right
        // away, rather than deferring it to `print_indent`. A group's fit
        // decision is made when its `Begin` token is printed, which happens
        // before any of its text (and thus before `print_indent` would run), so
        // the space must already reflect the indent or the group is measured
        // against the full margin and wrongly judged to fit.
        self.space = (MARGIN - self.print_indent * INDENT).max(MIN_SPACE);
        self.pending_break = false;
    }

    fn print_indent(&mut self) {
        if !self.line_dirty() {
            self.output
                .push_str(&" ".repeat((self.print_indent * INDENT).try_into().unwrap()));
        }
    }

    fn print_first(&mut self) {
        let token = self.tokens.pop_front().expect("no tokens to print");

        let group_break = self
            .print_frames
            .last()
            .is_some_and(|frame| frame.group_break);

        match &token {
            Token::Text(text_token) => {
                let should_print = matches!(
                    (text_token.mode(), group_break),
                    (TextMode::Always, _) | (TextMode::Break, true) | (TextMode::NoBreak, false)
                );
                if should_print {
                    self.print_string(text_token.string());
                }
            }
            Token::Break(break_token) => {
                if group_break || break_token.len() >= self.space {
                    self.print_indent = break_token.indent();
                    self.print_break();
                }
            }
            Token::ForceBreak => {
                self.pending_break = true;
            }
            Token::Begin(begin_token) => {
                let group_break =
                    begin_token.len() >= self.space && begin_token.mode() == BreakMode::Consistent;
                self.print_frames.push(PrintFrame { group_break });
            }
            Token::End => {
                self.print_frames.pop();
            }
        }
    }

    #[must_use]
    pub fn eof(mut self) -> String {
        while !self.tokens.is_empty() {
            self.print_first();
        }

        self.output
    }
}
