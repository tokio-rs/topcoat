use topcoat_runtime_coherence::coherent;

const STRINGS: &[&str] = &[
    "",
    "a",
    "ab",
    "abc",
    "b",
    "A",
    " ",
    "\t\r\n",
    "\0",
    "a\0b",
    "\"'\\<>&-->",
    "\u{00e9}",
    "e\u{0301}",
    "\u{1f980}",
    "\u{e000}",
    "\u{10000}",
    "\u{10ffff}",
    "\u{0085}hello\u{0085}",
    "\u{feff}hello\u{feff}",
    "\u{2028}\u{2029}",
    "\u{200b}",
    "\u{00a0} hello \u{3000}",
];

#[test]
fn string_method_and_comparison_matrix() {
    for &left in STRINGS {
        coherent!(left.len());
        coherent!(left.is_empty());
        coherent!(left.trim());
        coherent!(left.trim_start());
        coherent!(left.trim_end());
        coherent!(left.to_owned());
        for &right in STRINGS {
            coherent!(left == right);
            coherent!(left != right);
            coherent!(left < right);
            coherent!(left <= right);
            coherent!(left > right);
            coherent!(left >= right);
            coherent!(left.starts_with(right));
            coherent!(left.ends_with(right));
            coherent!(left.contains(right));
        }
    }
}

#[test]
fn owned_captures() {
    for &input in STRINGS {
        let text = input.to_owned();
        coherent!(text.len());
        coherent!(text.is_empty());
        coherent!(text.trim().to_owned());
        coherent!(text.trim_start().to_owned());
        coherent!(text.trim_end().to_owned());
        coherent!(direct => text);
    }
}

#[test]
fn literal_escaping() {
    coherent!("\0\"'\\\n\r\t<>&-->");
    coherent!("\u{2028}\u{2029}\u{1f980}");
    coherent!(r#"quotes " and a backslash \"#);
}
