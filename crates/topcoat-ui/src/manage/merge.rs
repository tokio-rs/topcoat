use std::borrow::Cow;

use diffy::{DiffOptions, Hunk, IncompleteHunkStyle, Line, MergeOptions};

/// The marker that opens a conflict, followed by the local lines.
const OURS_MARKER: &str = "<<<<<<< ours\n";
/// The marker between the local lines and the registry's lines.
const SEPARATOR_MARKER: &str = "=======\n";
/// The marker that closes a conflict, preceded by the registry's lines.
const THEIRS_MARKER: &str = ">>>>>>> theirs\n";

/// The line-ending style of a text file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    /// Detects the style of `text`. Text without a CRLF line ending counts as LF.
    pub(super) fn detect(text: &str) -> Self {
        if text.contains("\r\n") {
            Self::CrLf
        } else {
            Self::Lf
        }
    }

    /// Converts text with LF line endings to this style.
    pub(super) fn apply(self, text: String) -> String {
        match self {
            Self::Lf => text,
            Self::CrLf => text.replace('\n', "\r\n"),
        }
    }
}

/// Returns `text` with LF line endings.
pub(super) fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// The result of merging a registry source into a local file.
pub(super) struct Merged {
    /// The merged text, in the line-ending style of the local file.
    pub text: String,
    /// Whether `text` contains conflict markers that need resolving by hand.
    pub conflicted: bool,
}

/// Merges the registry's source `theirs` into the local file `ours`.
///
/// With `base`, the source both sides started from, changes made by only one side are
/// merged automatically and only overlapping changes become conflicts. Without it,
/// every region where the two sides differ becomes a conflict.
///
/// Line endings are ignored when comparing, and the result uses those of `ours`.
pub(super) fn merge(base: Option<&str>, ours: &str, theirs: &str) -> Merged {
    let line_ending = LineEnding::detect(ours);
    let ours = normalize(ours);
    let theirs = normalize(theirs);

    let (text, conflicted) = match base {
        Some(base) => three_way(&normalize(base), &ours, &theirs),
        None => two_way(&ours, &theirs),
    };
    Merged {
        text: line_ending.apply(text),
        conflicted,
    }
}

/// Merges two texts that share the ancestor `base`.
fn three_way(base: &str, ours: &str, theirs: &str) -> (String, bool) {
    let mut options = MergeOptions::new();
    // Keeps every marker on its own line when a side lacks a final newline.
    options.set_incomplete_hunk_style(IncompleteHunkStyle::Git);
    match options.merge(base, ours, theirs) {
        Ok(text) => (text, false),
        Err(text) => (text, true),
    }
}

/// Renders every region where `ours` and `theirs` differ as a conflict.
fn two_way(ours: &str, theirs: &str) -> (String, bool) {
    // With this much context the diff covers both texts in full, so the lines the two
    // sides share are part of it too.
    let mut options = DiffOptions::new();
    options.set_context_len(ours.lines().count() + theirs.lines().count());
    let patch = options.create_patch(ours, theirs);
    if patch.hunks().is_empty() {
        return (ours.to_string(), false);
    }

    let mut text = String::with_capacity(ours.len() + theirs.len());
    let mut conflict = Conflict::default();
    for line in patch.hunks().iter().flat_map(Hunk::lines) {
        match line {
            Line::Context(line) => {
                conflict.flush(&mut text);
                text.push_str(line);
            }
            Line::Delete(line) => conflict.ours.push_str(line),
            Line::Insert(line) => conflict.theirs.push_str(line),
        }
    }
    conflict.flush(&mut text);
    (text, true)
}

/// The differing lines of both sides collected for one conflict.
#[derive(Default)]
struct Conflict {
    ours: String,
    theirs: String,
}

impl Conflict {
    /// Writes the collected lines to `text` between conflict markers, then clears
    /// them. Does nothing if no lines were collected.
    fn flush(&mut self, text: &mut String) {
        if self.ours.is_empty() && self.theirs.is_empty() {
            return;
        }
        text.push_str(OURS_MARKER);
        push_lines(text, &self.ours);
        text.push_str(SEPARATOR_MARKER);
        push_lines(text, &self.theirs);
        text.push_str(THEIRS_MARKER);
        self.ours.clear();
        self.theirs.clear();
    }
}

/// Appends `lines` to `text`, ending the last line if it lacks a newline so that the
/// next marker starts on its own line.
fn push_lines(text: &mut String, lines: &str) {
    text.push_str(lines);
    if !lines.is_empty() && !lines.ends_with('\n') {
        text.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "one\ntwo\nthree\nfour\nfive\n";

    /// Resolves every conflict in `text` to one side.
    fn resolve(text: &str, take_ours: bool) -> String {
        let mut resolved = String::new();
        let mut in_ours = None;
        for line in text.split_inclusive('\n') {
            match line {
                OURS_MARKER => in_ours = Some(true),
                SEPARATOR_MARKER => in_ours = Some(false),
                THEIRS_MARKER => in_ours = None,
                _ if in_ours.is_none_or(|in_ours| in_ours == take_ours) => resolved.push_str(line),
                _ => {}
            }
        }
        resolved
    }

    #[test]
    fn changes_on_different_lines_merge_without_conflict() {
        let ours = BASE.replace("two", "TWO");
        let theirs = BASE.replace("four", "FOUR");

        let merged = merge(Some(BASE), &ours, &theirs);

        assert!(!merged.conflicted);
        assert_eq!(merged.text, "one\nTWO\nthree\nFOUR\nfive\n");
    }

    #[test]
    fn changes_on_the_same_line_conflict() {
        let ours = BASE.replace("two", "OURS");
        let theirs = BASE.replace("two", "THEIRS");

        let merged = merge(Some(BASE), &ours, &theirs);

        assert!(merged.conflicted);
        assert!(merged.text.contains("OURS\n") && merged.text.contains("THEIRS\n"));
    }

    #[test]
    fn merging_without_a_base_keeps_both_sides_recoverable() {
        // Neither side ends in a newline, which must not glue a marker to a line.
        let ours = "one\nTWO\nthree\nfour\nextra";
        let theirs = "one\ntwo\nthree\nFOUR\nfive";

        let merged = merge(None, ours, theirs);

        assert!(merged.conflicted);
        assert_eq!(resolve(&merged.text, true).trim_end(), ours);
        assert_eq!(resolve(&merged.text, false).trim_end(), theirs);
        // Lines both sides share stay outside the conflicts.
        assert_eq!(merged.text.matches("three\n").count(), 1);
    }

    #[test]
    fn line_endings_do_not_conflict_and_follow_the_local_file() {
        let ours = BASE.replace("two", "TWO").replace('\n', "\r\n");
        let theirs = BASE.replace("four", "FOUR");

        let merged = merge(Some(BASE), &ours, &theirs);

        assert!(!merged.conflicted);
        assert_eq!(merged.text, "one\r\nTWO\r\nthree\r\nFOUR\r\nfive\r\n");
    }
}
