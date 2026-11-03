use proc_macro2::{Delimiter, Ident, Punct, Spacing, TokenTree};
use syn::buffer::Cursor;

/// Punctuation tokens of more than one character, longest first so that `..=`
/// is not read as `..`.
const LONG_PUNCTS: &[&str] = &[
    "<<=", ">>=", "..=", "::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=",
    "/=", "%=", "^=", "&=", "|=", "<<", ">>", "..",
];

/// Punctuation tokens of a single character.
const SHORT_PUNCTS: &[&str] = &[
    "+", "-", "*", "/", "%", "^", "!", "&", "|", "=", "<", ">", "@", ".", ",", ";", ":", "#", "$",
    "?", "~",
];

/// Operators that join two operands.
const BINARY_OPERATORS: &[&str] = &[
    "<<=", ">>=", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "^=", "&=",
    "|=", "<<", ">>", "+", "-", "*", "/", "%", "^", "&", "|", "=", "<", ">",
];

/// Words that cannot be used as a name without `r#`.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Keywords that can start or appear in a path.
const PATH_KEYWORDS: &[&str] = &["self", "Self", "super", "crate"];

/// Finds where a Rust expression ends by reading whole token trees.
///
/// The scanner never looks inside a delimited group and does not check that
/// the expression is valid. It reads an operand, keeps reading while the next
/// token can continue the expression, and stops at the first token that
/// cannot. Incomplete code follows the same rule, so `user.` ends after the
/// `.` and `a +` after the `+`.
///
/// Operator precedence decides how an expression groups, not where it ends,
/// so the scanner does not track it.
#[derive(Clone, Copy)]
pub(super) struct Scanner {
    /// Whether `{ ... }` can start an operand or follow a path as a struct
    /// literal. It is off in the condition of an `if`, `match`, `while` or
    /// `for`, where it starts the body instead.
    allow_brace: bool,
}

impl Scanner {
    pub(super) fn new(allow_brace: bool) -> Self {
        Self { allow_brace }
    }

    /// Returns the position after the expression at `cursor`, or `cursor`
    /// itself when no expression starts there.
    pub(super) fn expr(self, cursor: Cursor<'_>) -> Cursor<'_> {
        match self.operand(cursor) {
            Some(next) => self.continuation(next),
            None => cursor,
        }
    }

    /// Reads an operand along with its prefix operators. Returns `None` when
    /// no operand starts at `cursor`.
    fn operand(self, cursor: Cursor<'_>) -> Option<Cursor<'_>> {
        if let Some(next) = eat(cursor, "-")
            .or_else(|| eat(cursor, "!"))
            .or_else(|| eat(cursor, "*"))
        {
            return Some(self.operand(next).unwrap_or(next));
        }
        if let Some(next) = eat(cursor, "&").or_else(|| eat(cursor, "&&")) {
            let next = keyword(next, "raw")
                .and_then(|raw| keyword(raw, "const").or_else(|| keyword(raw, "mut")))
                .or_else(|| keyword(next, "mut"))
                .unwrap_or(next);
            return Some(self.operand(next).unwrap_or(next));
        }
        if let Some(next) = eat(cursor, "..").or_else(|| eat(cursor, "..=")) {
            return Some(self.expr(next));
        }
        if eat(cursor, "|").is_some() || eat(cursor, "||").is_some() {
            return Some(self.closure(cursor));
        }
        if let Some(label) = lifetime(cursor) {
            let next = eat(label, ":")?;
            return Some(
                group(next, Delimiter::Brace)
                    .or_else(|| self.operand(next))
                    .unwrap_or(next),
            );
        }
        if let Some(next) = literal(cursor)
            .or_else(|| group(cursor, Delimiter::Parenthesis))
            .or_else(|| group(cursor, Delimiter::Bracket))
            .or_else(|| group(cursor, Delimiter::None))
        {
            return Some(next);
        }
        if self.allow_brace
            && let Some(next) = group(cursor, Delimiter::Brace)
        {
            return Some(next);
        }
        let Some((ident, next)) = ident(cursor) else {
            return self.path_operand(cursor);
        };
        match ident.to_string().as_str() {
            "if" => Some(Self::if_expr(next)),
            "match" | "while" => Some(Self::condition_and_body(next)),
            "for" => Some(Self::for_expr(next)),
            "loop" | "unsafe" | "const" => Some(group(next, Delimiter::Brace).unwrap_or(next)),
            "async" => {
                let next = keyword(next, "move").unwrap_or(next);
                Some(group(next, Delimiter::Brace).unwrap_or_else(|| self.closure(next)))
            }
            "move" | "static" => Some(self.closure(next)),
            "return" | "yield" => Some(self.expr(next)),
            "break" => Some(self.expr(lifetime(next).unwrap_or(next))),
            "continue" => Some(lifetime(next).unwrap_or(next)),
            "let" => Some(self.let_expr(next)),
            "true" | "false" => Some(next),
            _ => self.path_operand(cursor),
        }
    }

    /// Reads what follows an operand: postfix operators, calls, casts and
    /// binary operators with their right operand.
    fn continuation(self, mut cursor: Cursor<'_>) -> Cursor<'_> {
        loop {
            if let Some(next) =
                group(cursor, Delimiter::Parenthesis).or_else(|| group(cursor, Delimiter::Bracket))
            {
                cursor = next;
                continue;
            }
            if let Some(next) = keyword(cursor, "as") {
                let Some(next) = ty(next) else { return next };
                cursor = next;
                continue;
            }
            let Some((token, next)) = punct(cursor) else {
                return cursor;
            };
            cursor = match token {
                "?" => next,
                "." => match member(next) {
                    Some(next) => next,
                    None => return next,
                },
                ".." | "..=" => return self.expr(next),
                _ if BINARY_OPERATORS.contains(&token) => match self.operand(next) {
                    Some(next) => next,
                    None => return next,
                },
                _ => return cursor,
            };
        }
    }

    /// Reads a path operand, along with the arguments of a macro call or the
    /// fields of a struct literal.
    fn path_operand(self, cursor: Cursor<'_>) -> Option<Cursor<'_>> {
        let next = path(cursor, false)?;
        if let Some(next) = eat(next, "!").and_then(|bang| {
            group(bang, Delimiter::Parenthesis)
                .or_else(|| group(bang, Delimiter::Bracket))
                .or_else(|| group(bang, Delimiter::Brace))
        }) {
            return Some(next);
        }
        if self.allow_brace
            && let Some(next) = group(next, Delimiter::Brace)
        {
            return Some(next);
        }
        Some(next)
    }

    /// Reads a closure from its `async` or `move` to the end of its body.
    fn closure(self, mut cursor: Cursor<'_>) -> Cursor<'_> {
        while let Some(next) = keyword(cursor, "async").or_else(|| keyword(cursor, "move")) {
            cursor = next;
        }
        let params_end = if let Some(next) = eat(cursor, "||") {
            next
        } else if let Some(next) = eat(cursor, "|") {
            closure_params(next)
        } else {
            return cursor;
        };
        if let Some(next) = eat(params_end, "->") {
            let next = ty(next).unwrap_or(next);
            return group(next, Delimiter::Brace).unwrap_or(next);
        }
        self.expr(params_end)
    }

    /// Reads a `let` from its pattern to the end of its value.
    fn let_expr(self, cursor: Cursor<'_>) -> Cursor<'_> {
        let pattern_end = pattern(cursor, |cursor| eat(cursor, "=").is_some());
        match eat(pattern_end, "=") {
            Some(next) => self.expr(next),
            None => pattern_end,
        }
    }

    /// Reads an `if` from its condition to the end of its last `else`.
    fn if_expr(cursor: Cursor<'_>) -> Cursor<'_> {
        let condition = Self::new(false).expr(cursor);
        let Some(body) = group(condition, Delimiter::Brace) else {
            return condition;
        };
        let Some(next) = keyword(body, "else") else {
            return body;
        };
        if let Some(next) = keyword(next, "if") {
            return Self::if_expr(next);
        }
        group(next, Delimiter::Brace).unwrap_or(next)
    }

    /// Reads a `for` from its pattern to the end of its body.
    fn for_expr(cursor: Cursor<'_>) -> Cursor<'_> {
        let pattern_end = pattern(cursor, |cursor| keyword(cursor, "in").is_some());
        match keyword(pattern_end, "in") {
            Some(next) => Self::condition_and_body(next),
            None => pattern_end,
        }
    }

    /// Reads the condition or scrutinee of a `match` or `while` and the body
    /// that follows it.
    fn condition_and_body(cursor: Cursor<'_>) -> Cursor<'_> {
        let condition = Self::new(false).expr(cursor);
        group(condition, Delimiter::Brace).unwrap_or(condition)
    }
}

/// Reads a path like `a::b`, `::a`, `<T as Trait>::a` or `Vec::<u8>::new`.
/// In a type, generics also follow a segment without `::`, as in `Vec<u8>`
/// and `Fn(u8) -> u8`.
fn path(cursor: Cursor<'_>, in_type: bool) -> Option<Cursor<'_>> {
    let mut next = if let Some(qself) = open_generics(cursor) {
        match eat(qself, "::") {
            Some(next) => next,
            None => return Some(qself),
        }
    } else {
        eat(cursor, "::").unwrap_or(cursor)
    };
    loop {
        let Some(segment) = ident(next)
            .filter(|(ident, _)| is_path_segment(ident))
            .map(|(_, next)| next)
        else {
            return (next != cursor).then_some(next);
        };
        next = segment;
        if in_type {
            if let Some(args) = open_generics(next) {
                next = args;
            } else if let Some(inputs) = group(next, Delimiter::Parenthesis) {
                next = return_type(inputs);
            }
        }
        let Some(separator) = eat(next, "::") else {
            return Some(next);
        };
        next = separator;
        if let Some(args) = open_generics(next) {
            next = args;
            match eat(next, "::") {
                Some(separator) => next = separator,
                None => return Some(next),
            }
        }
    }
}

/// Reads a type, as after `as` or a closure's `->`. As after `as`, a `+` does
/// not continue the type.
fn ty(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    if let Some(next) = eat(cursor, "&").or_else(|| eat(cursor, "&&")) {
        let next = lifetime(next).unwrap_or(next);
        let next = keyword(next, "mut").unwrap_or(next);
        return Some(ty(next).unwrap_or(next));
    }
    if let Some(next) = eat(cursor, "*") {
        let next = keyword(next, "const")
            .or_else(|| keyword(next, "mut"))
            .unwrap_or(next);
        return Some(ty(next).unwrap_or(next));
    }
    if let Some(next) = group(cursor, Delimiter::Parenthesis)
        .or_else(|| group(cursor, Delimiter::Bracket))
        .or_else(|| group(cursor, Delimiter::None))
        .or_else(|| eat(cursor, "!"))
        .or_else(|| keyword(cursor, "_"))
    {
        return Some(next);
    }
    if let Some(next) = keyword(cursor, "dyn").or_else(|| keyword(cursor, "impl")) {
        let next = eat(next, "?").unwrap_or(next);
        return Some(path(next, true).unwrap_or(next));
    }
    fn_pointer(cursor).or_else(|| path(cursor, true))
}

/// Reads a function pointer type like `unsafe extern "C" fn(u8) -> u8`.
fn fn_pointer(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    let mut next = cursor;
    if let Some(after) = keyword(next, "for") {
        next = open_generics(after).unwrap_or(after);
    }
    if let Some(after) = keyword(next, "unsafe") {
        next = after;
    }
    if let Some(after) = keyword(next, "extern") {
        next = literal(after).unwrap_or(after);
    }
    let Some(after) = keyword(next, "fn") else {
        return (next != cursor).then_some(next);
    };
    Some(group(after, Delimiter::Parenthesis).map_or(after, return_type))
}

/// Reads an optional `-> Type`.
fn return_type(cursor: Cursor<'_>) -> Cursor<'_> {
    eat(cursor, "->").map_or(cursor, |next| ty(next).unwrap_or(next))
}

/// Reads a field, tuple index, `await` or method name after a `.`. Returns
/// `None` when none follows.
fn member(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    if let Some(next) = literal(cursor) {
        return Some(next);
    }
    let (ident, next) = ident(cursor)?;
    let name = ident.to_string();
    if name == "await" {
        return Some(next);
    }
    if KEYWORDS.contains(&name.as_str()) {
        return None;
    }
    let Some(separator) = eat(next, "::") else {
        return Some(next);
    };
    Some(open_generics(separator).unwrap_or(separator))
}

/// Reads generic arguments from their opening `<` to the `>` that closes
/// them.
fn open_generics(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    if let Some(next) = eat(cursor, "<") {
        Some(generic_args(next, 1))
    } else {
        eat(cursor, "<<").map(|next| generic_args(next, 2))
    }
}

/// Reads generic arguments up to the `>` that closes `depth` open `<`s.
/// Commas are valid inside them, so unclosed arguments end before a `;` or
/// `=>` instead.
fn generic_args(mut cursor: Cursor<'_>, mut depth: usize) -> Cursor<'_> {
    // Whether the previous token starts a `->` or `=>`, whose `>` closes
    // nothing.
    let mut arrow = false;
    while !ends_unclosed(cursor) {
        let Some((token, next)) = cursor.token_tree() else {
            break;
        };
        if let TokenTree::Punct(punct) = &token {
            match punct.as_char() {
                '<' => depth += 1,
                '>' if !arrow => {
                    depth -= 1;
                    if depth == 0 {
                        return next;
                    }
                }
                _ => {}
            }
            arrow = matches!(punct.as_char(), '-' | '=') && punct.spacing() == Spacing::Joint;
        } else {
            arrow = false;
        }
        cursor = next;
    }
    cursor
}

/// Reads a closure's parameters up to the `|` that closes them. Commas are
/// valid inside them, so unclosed parameters end before a `;` or `=>`
/// instead.
fn closure_params(mut cursor: Cursor<'_>) -> Cursor<'_> {
    while !ends_unclosed(cursor) {
        let Some((token, next)) = cursor.token_tree() else {
            break;
        };
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == '|') {
            return next;
        }
        cursor = next;
    }
    cursor
}

/// Reads a pattern up to the token `is_end` accepts, or up to a `,`, `;` or
/// `=>` when the pattern is incomplete.
fn pattern(mut cursor: Cursor<'_>, is_end: fn(Cursor<'_>) -> bool) -> Cursor<'_> {
    while !ends_unclosed(cursor) && !is_end(cursor) && eat(cursor, ",").is_none() {
        cursor = match punct(cursor) {
            Some((_, next)) => next,
            None => match cursor.token_tree() {
                Some((_, next)) => next,
                None => break,
            },
        };
    }
    cursor
}

/// Checks whether an unclosed list of generic arguments or closure
/// parameters ends at `cursor`.
fn ends_unclosed(cursor: Cursor<'_>) -> bool {
    cursor.eof() || matches!(punct(cursor), Some((";" | "=>", _)))
}

/// Checks whether `ident` can be a segment of a path.
fn is_path_segment(ident: &Ident) -> bool {
    let name = ident.to_string();
    !KEYWORDS.contains(&name.as_str()) || PATH_KEYWORDS.contains(&name.as_str())
}

/// Reads the punctuation token at `cursor`, preferring the longest one.
fn punct(cursor: Cursor<'_>) -> Option<(&'static str, Cursor<'_>)> {
    let (first, _) = punct_tree(cursor)?;
    LONG_PUNCTS
        .iter()
        .chain(SHORT_PUNCTS)
        .filter(|token| token.starts_with(first.as_char()))
        .find_map(|token| eat_chars(cursor, token).map(|next| (*token, next)))
}

/// Reads the punctuation token `token`, but not the start of a longer one.
fn eat<'a>(cursor: Cursor<'a>, token: &str) -> Option<Cursor<'a>> {
    punct(cursor)
        .filter(|(found, _)| *found == token)
        .map(|(_, next)| next)
}

/// Reads the characters of `token`, each joined to the next.
fn eat_chars<'a>(mut cursor: Cursor<'a>, token: &str) -> Option<Cursor<'a>> {
    let mut chars = token.chars().peekable();
    while let Some(char) = chars.next() {
        let (punct, next) = punct_tree(cursor)?;
        let joined = chars.peek().is_none() || punct.spacing() == Spacing::Joint;
        if punct.as_char() != char || !joined {
            return None;
        }
        cursor = next;
    }
    Some(cursor)
}

/// Reads the identifier `name`.
fn keyword<'a>(cursor: Cursor<'a>, name: &str) -> Option<Cursor<'a>> {
    ident(cursor)
        .filter(|(ident, _)| ident == name)
        .map(|(_, next)| next)
}

// The readers below treat a `None`-delimited group as a single token tree,
// while the `Cursor` methods they call would look into it.

fn punct_tree(cursor: Cursor<'_>) -> Option<(Punct, Cursor<'_>)> {
    cursor.any_group().is_none().then(|| cursor.punct())?
}

fn ident(cursor: Cursor<'_>) -> Option<(Ident, Cursor<'_>)> {
    cursor.any_group().is_none().then(|| cursor.ident())?
}

fn literal(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    cursor
        .any_group()
        .is_none()
        .then(|| cursor.literal())?
        .map(|(_, next)| next)
}

fn lifetime(cursor: Cursor<'_>) -> Option<Cursor<'_>> {
    cursor
        .any_group()
        .is_none()
        .then(|| cursor.lifetime())?
        .map(|(_, next)| next)
}

fn group(cursor: Cursor<'_>, delimiter: Delimiter) -> Option<Cursor<'_>> {
    let (_, found, _, next) = cursor.any_group()?;
    (found == delimiter).then_some(next)
}
