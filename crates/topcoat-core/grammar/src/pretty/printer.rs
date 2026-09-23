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

/// The pretty-printing engine.
///
/// This is an Oppen-style printer. Callers feed it a stream of text, breaks,
/// and group boundaries with the `scan_*` methods, and the printer decides
/// which breaks become line breaks based on the available width. Call
/// [`eof`](Self::eof) to get the output.
///
/// The printer also tracks a cursor in the original source. Moving the
/// cursor past comments and blank lines lets the `scan_*trivia` methods
/// reproduce them in the output.
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
    /// Creates a printer.
    ///
    /// `trivia` holds the comments and whitespace of the source being
    /// printed, in order. `initial_space` is the width available on the first
    /// line, and `initial_indent` the indentation level the output starts at.
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

    /// Returns the registry of macro pretty-printers, for printing nested
    /// macro invocations.
    #[must_use]
    pub fn registry(&self) -> &'a Registry {
        self.registry
    }

    /// Returns the current position of the cursor in the source.
    #[must_use]
    pub fn cursor(&self) -> LineColumn {
        self.cursor
    }

    /// Moves the cursor to `cursor` in the source.
    pub fn move_cursor(&mut self, cursor: LineColumn) {
        self.cursor = cursor;
    }

    /// Moves the cursor forward over `string`, as if it had been read from
    /// the source.
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

    /// Adds text to the output, printed or not depending on `mode`.
    ///
    /// # Panics
    ///
    /// Panics if `string.len()` does not fit in an `isize`.
    pub fn scan_text(&mut self, string: Cow<'static, str>, mode: TextMode) {
        // Break-mode text (a trailing comma) renders only once its group has
        // broken, so it takes no space in the flat layout being measured.
        if mode != TextMode::Break {
            self.tokens.push_len(string.len().try_into().unwrap());
        }
        let token = Token::Text(TextToken::new(string, mode));
        self.tokens.push_back(token);
    }

    /// Adds a break, which becomes a line break or prints nothing, depending
    /// on the enclosing group's [`BreakMode`] and the available width.
    pub fn scan_break(&mut self) {
        self.tokens
            .push_back(Token::Break(BreakToken::new(0, self.scan_indent)));
    }

    /// Adds a line break that is always taken. It also counts as too wide to
    /// fit, so the enclosing consistent groups break as well.
    pub fn scan_force_break(&mut self) {
        let len = MARGIN;
        self.tokens.push_back(Token::ForceBreak);
        self.tokens.push_len(len);
    }

    /// Changes the indentation level of the breaks added after this call by
    /// `indent` levels.
    pub fn scan_indent(&mut self, indent: isize) {
        self.scan_indent += indent;
    }

    /// Returns the current indentation level.
    #[must_use]
    pub fn current_indent(&self) -> isize {
        self.scan_indent
    }

    /// Starts a group with the given break mode. End it with
    /// [`scan_end`](Self::scan_end).
    pub fn scan_begin(&mut self, mode: BreakMode) {
        self.tokens
            .push_back(Token::Begin(BeginToken::new(mode, 0)));
    }

    /// Ends the group started by the matching [`scan_begin`](Self::scan_begin).
    ///
    /// # Panics
    ///
    /// Panics if there is no open group.
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

    /// Prints the block comments before the cursor for a place where no line
    /// break may be added. Stops at a line comment, which needs a line break
    /// after it.
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

    /// Prints the first comment before the cursor if it sits on the current
    /// source line, like a trailing `// comment` after a statement.
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

    /// Prints the comments before the cursor, each on its own line, and keeps
    /// blank lines between them.
    ///
    /// `leading_whitespace` keeps a blank line and a separating space before
    /// the first comment, and `trailing_whitespace` keeps a blank line after
    /// the last one.
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

    /// Drops the trivia that starts before the cursor without printing it,
    /// for callers that copied a stretch of source text as it is. Trivia that
    /// starts exactly at the cursor lies outside the copied text and is kept.
    pub fn skip_trivia(&mut self) {
        while let Some(trivia) = self.trivia.first() {
            if trivia.span.start() < self.cursor {
                self.trivia = &self.trivia[1..];
            } else {
                break;
            }
        }
    }

    /// Returns whether a comment that was not printed yet starts before `pos`
    /// in the source.
    ///
    /// This lets an otherwise empty construct tell a comment inside it apart
    /// from plain whitespace.
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

    /// Prints everything that was added and returns the output.
    #[must_use]
    pub fn eof(mut self) -> String {
        while !self.tokens.is_empty() {
            self.print_first();
        }

        self.output
    }
}
