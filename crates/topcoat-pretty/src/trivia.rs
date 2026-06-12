use proc_macro2::LineColumn;

use super::Span;

/// The kind of source-text trivia captured by [`Lexer`].
#[derive(Debug, Clone, PartialEq)]
pub enum TriviaKind {
    LineComment,
    BlockComment,
    Whitespace,
}

/// A single chunk of comment or whitespace, with its original source span.
///
/// The pretty printer threads these through so that comments and significant
/// blank lines from the source are reproduced in the formatted output.
#[derive(Debug, Clone, PartialEq)]
pub struct Trivia<'a> {
    pub span: Span,
    pub content: &'a str,
    pub kind: TriviaKind,
}

impl Trivia<'_> {
    /// The number of `\n` characters contained in the trivia's source text.
    #[must_use]
    pub fn newlines(&self) -> usize {
        self.content.chars().filter(|item| *item == '\n').count()
    }
}

/// A lexer that skips code tokens but captures comments and whitespace.
pub struct Lexer<'a> {
    input: &'a str,
    // Current byte offset in the input string
    cursor: usize,
    // Current line number (1-based)
    line: usize,
    // Current column number (0-based)
    column: usize,
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor: 0,
            line: 1,
            column: 0,
        }
    }

    /// Returns the remaining string slice from the current cursor
    fn rest(&self) -> &'a str {
        &self.input[self.cursor..]
    }

    /// Peeks at the next character without advancing
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Peeks at the character after the next one
    fn peek_second(&self) -> Option<char> {
        let mut chars = self.rest().chars();
        chars.next();
        chars.next()
    }

    /// Advances the cursor by one character (utf-8 aware)
    /// Updates line/col tracking.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        let len = c.len_utf8();

        self.cursor += len;

        if c == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }

        Some(c)
    }

    fn current_pos(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    /// Scans whitespace and returns a Token
    fn scan_whitespace(&mut self) -> Trivia<'a> {
        let start_idx = self.cursor;
        let (s_line, s_col) = self.current_pos();

        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }

        let content = &self.input[start_idx..self.cursor];
        Trivia {
            span: Span::new(
                LineColumn {
                    line: s_line,
                    column: s_col,
                },
                LineColumn {
                    line: self.line,
                    column: self.column,
                },
            ),
            content,
            kind: TriviaKind::Whitespace,
        }
    }

    /// Scans a line comment (// ...)
    fn scan_line_comment(&mut self) -> Trivia<'a> {
        let start_idx = self.cursor;
        let (s_line, s_col) = self.current_pos();

        // consume //
        self.bump();
        self.bump();

        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.bump();
        }

        let content = &self.input[start_idx..self.cursor];
        Trivia {
            span: Span::new(
                LineColumn {
                    line: s_line,
                    column: s_col,
                },
                LineColumn {
                    line: self.line,
                    column: self.column,
                },
            ),
            content,
            kind: TriviaKind::LineComment,
        }
    }

    /// Scans a block comment (/* ... */), handling nesting
    fn scan_block_comment(&mut self) -> Trivia<'a> {
        let start_idx = self.cursor;
        let (s_line, s_col) = self.current_pos();

        self.bump(); // /
        self.bump(); // *

        let mut depth = 1;

        while depth > 0 {
            match (self.peek(), self.peek_second()) {
                (Some('/'), Some('*')) => {
                    depth += 1;
                    self.bump();
                    self.bump();
                }
                (Some('*'), Some('/')) => {
                    depth -= 1;
                    self.bump();
                    self.bump();
                }
                (Some(_), _) => {
                    self.bump();
                }
                (None, _) => break, // Unexpected EOF, but return what we have
            }
        }

        let content = &self.input[start_idx..self.cursor];
        Trivia {
            span: Span::new(
                LineColumn {
                    line: s_line,
                    column: s_col,
                },
                LineColumn {
                    line: self.line,
                    column: self.column,
                },
            ),
            content,
            kind: TriviaKind::BlockComment,
        }
    }

    /// Skips a standard string literal "..." handling escapes \"
    fn skip_string(&mut self) {
        self.bump(); // Consume opening "
        while let Some(c) = self.peek() {
            match c {
                '"' => {
                    self.bump();
                    return;
                }
                '\\' => {
                    self.bump(); // consume backslash
                    self.bump(); // consume escaped char
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Skips a char literal '...' handling escapes \'
    fn skip_char(&mut self) {
        self.bump(); // Consume opening '
        while let Some(c) = self.peek() {
            match c {
                '\'' => {
                    self.bump();
                    return;
                }
                '\\' => {
                    self.bump();
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Skips a raw string r#"..."#
    /// Logic: r, optional #, ", content, ", matching #
    fn skip_raw_string(&mut self) {
        // 1. Count opening hashes
        let mut hashes = 0;
        while let Some('#') = self.peek() {
            hashes += 1;
            self.bump();
        }

        // 2. Consume opening quote
        if let Some('"') = self.peek() {
            self.bump();
        } else {
            return; // Not a string
        }

        // 3. Find closing sequence: " + # * hashes
        // We loop char by char to update spans correctly
        loop {
            match self.peek() {
                Some('"') => {
                    // Check if followed by N hashes
                    let potential_hashes = &self.input[self.cursor + 1..]; // +1 for the "
                    let mut matches = true;
                    if potential_hashes.len() < hashes {
                        matches = false;
                    } else {
                        for i in 0..hashes {
                            if potential_hashes.as_bytes()[i] != b'#' {
                                matches = false;
                                break;
                            }
                        }
                    }

                    if matches {
                        self.bump(); // consume "
                        for _ in 0..hashes {
                            self.bump();
                        } // consume #
                        return;
                    }
                    self.bump(); // Just a quote inside string
                }
                Some(_) => {
                    self.bump();
                }
                None => return,
            }
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Trivia<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let c = self.peek()?;

            if c.is_whitespace() {
                return Some(self.scan_whitespace());
            }

            // Check for comments
            if c == '/'
                && let Some(next) = self.peek_second()
            {
                if next == '/' {
                    return Some(self.scan_line_comment());
                } else if next == '*' {
                    return Some(self.scan_block_comment());
                }
            }

            // We need to skip code (strings, chars, identifiers) to find the next comment
            // Crucially, we must skip strings/chars correctly so we don't find false comments
            // inside them.
            match c {
                '"' => self.skip_string(),
                '\'' => self.skip_char(),
                'b' => {
                    // Could be b"" or br"" or b'' or just identifier 'bytes'
                    match self.peek_second() {
                        Some('"') => {
                            self.bump();
                            self.skip_string();
                        }
                        Some('\'') => {
                            self.bump();
                            self.skip_char();
                        }
                        Some('r') => {
                            // could be br" or br#
                            // consume b
                            self.bump();
                            // consume r
                            self.bump();
                            self.skip_raw_string();
                        }
                        _ => {
                            self.bump();
                        }
                    }
                }
                'r' => {
                    // Could be r" or r# or identifier 'rust'
                    match self.peek_second() {
                        Some('"' | '#') => {
                            self.bump(); // consume r
                            self.skip_raw_string();
                        }
                        _ => {
                            self.bump();
                        }
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_tokens(input: &str) -> Vec<Trivia<'_>> {
        Lexer::new(input).collect()
    }

    #[test]
    fn test_simple_comments() {
        let src = "let x = 5; // hello\n/* world */";
        let tokens = collect_tokens(src);

        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].kind, TriviaKind::Whitespace); // " " after let
        assert_eq!(tokens[1].kind, TriviaKind::Whitespace); // " " after x
        assert_eq!(tokens[2].kind, TriviaKind::Whitespace); // " " after =
        assert_eq!(tokens[3].kind, TriviaKind::Whitespace); // " " after ;
        assert_eq!(tokens[4].kind, TriviaKind::LineComment); // "// hello"
        assert_eq!(tokens[4].content, "// hello");
        assert_eq!(tokens[5].kind, TriviaKind::Whitespace); // "\n"
        assert_eq!(tokens[6].kind, TriviaKind::BlockComment); // "/* world */"
        assert_eq!(tokens[6].content, "/* world */"); // "/* world */"
    }

    #[test]
    fn test_ignore_comments_in_strings() {
        let src = r#"let s = "// not a comment"; // real comment"#;
        let tokens = collect_tokens(src);

        // Whitespace (spaces around s and =) are captured as they are found
        // But wait: "let", "x", "=", ";" are consumed by the loop as code and discarded.
        // So we only see WHITESPACE and COMMENTS.

        // 1. "let" -> skipped
        // 2. " " -> Token
        // 3. "s" -> skipped
        // 4. " " -> Token
        // 5. "=" -> skipped
        // 6. " " -> Token
        // 7. string literal -> skipped entirely
        // 8. ";" -> skipped
        // 9. " " -> Token
        // 10. "// real comment" -> Token

        let comments: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TriviaKind::LineComment))
            .collect();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].content, "// real comment");
    }

    #[test]
    fn test_nested_block_comments() {
        let src = "/* outer /* inner */ outer */ code";
        let tokens = collect_tokens(src);
        assert_eq!(tokens[0].kind, TriviaKind::BlockComment);
        assert_eq!(tokens[0].content, "/* outer /* inner */ outer */");
    }

    #[test]
    fn test_raw_strings() {
        // This contains a "comment" inside a raw string that should be ignored
        let src = r##" r#" /* fake */ "# "##; // The raw string is `r#" /* fake */ "#`
        let tokens = collect_tokens(src);
        // Should be empty or just whitespace depending on spaces
        assert!(tokens.iter().all(|t| t.kind == TriviaKind::Whitespace));
    }

    #[test]
    fn test_spans() {
        let src = "Code\n// Com";
        let tokens = collect_tokens(src);
        // Token 0: \n (Whitespace)
        // Token 1: // Com (LineComment)

        // "Code" is 4 chars. Line 1, cols 0-3. Skipped.
        // "\n" is at Line 1, col 4.
        // "// Com" starts Line 2, col 0.

        let newline = &tokens[0];
        assert_eq!(newline.kind, TriviaKind::Whitespace);
        assert_eq!(newline.span.start().line, 1);
        assert_eq!(newline.span.end().line, 2); // Ends after newline

        let comment = &tokens[1];
        assert_eq!(comment.kind, TriviaKind::LineComment);
        assert_eq!(comment.span.start().line, 2);
        assert_eq!(comment.span.start().column, 0);
    }
}
