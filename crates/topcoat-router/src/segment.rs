use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
    Static(&'a str),
    Group(&'a str),
    Param(&'a str),
    CatchAll(&'a str),
}

impl<'a> Segment<'a> {
    pub fn new(s: &'a str) -> Self {
        if s.starts_with('{') {
            if !s.ends_with('}') {
                panic!("invalid segment: missing closing `}}` in `{s}`");
            }
            let inner = &s[1..s.len() - 1];
            if let Some(name) = inner.strip_prefix('*') {
                assert_valid_ident(name, "catch-all", s);
                Segment::CatchAll(name)
            } else {
                assert_valid_ident(inner, "param", s);
                Segment::Param(inner)
            }
        } else if s.starts_with('(') {
            if !s.ends_with(')') {
                panic!("invalid segment: missing closing `)` in `{s}`");
            }
            let inner = &s[1..s.len() - 1];
            assert_valid_ident(inner, "group", s);
            Segment::Group(inner)
        } else {
            if s.is_empty() {
                panic!("invalid segment: segment must not be empty");
            }
            if s.contains('{') || s.contains('}') || s.contains('(') || s.contains(')') {
                panic!("invalid segment: unexpected brackets in `{s}`");
            }
            Segment::Static(s)
        }
    }

    /// Returns `true` if the segment is [`Static`].
    ///
    /// [`Static`]: Segment::Static
    #[must_use]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static(..))
    }

    /// Returns `true` if the segment is [`Group`].
    ///
    /// [`Group`]: Segment::Group
    #[must_use]
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(..))
    }

    /// Returns `true` if the segment is [`Param`].
    ///
    /// [`Param`]: Segment::Param
    #[must_use]
    pub fn is_param(&self) -> bool {
        matches!(self, Self::Param(..))
    }

    /// Returns `true` if the segment is [`CatchAll`].
    ///
    /// [`CatchAll`]: Segment::CatchAll
    #[must_use]
    pub fn is_catch_all(&self) -> bool {
        matches!(self, Self::CatchAll(..))
    }

    pub fn as_static(&self) -> Option<&&'a str> {
        if let Self::Static(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_group(&self) -> Option<&&'a str> {
        if let Self::Group(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_param(&self) -> Option<&&'a str> {
        if let Self::Param(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_catch_all(&self) -> Option<&&'a str> {
        if let Self::CatchAll(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

fn assert_valid_ident(name: &str, kind: &str, raw: &str) {
    if name.is_empty() {
        panic!("invalid segment: {kind} name must not be empty in `{raw}`");
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        panic!(
            "invalid segment: {kind} name `{name}` must start with a letter or underscore in `{raw}`"
        );
    }
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            panic!(
                "invalid segment: {kind} name `{name}` contains invalid character `{ch}` in `{raw}`"
            );
        }
    }
}

impl Display for Segment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(inner) => f.write_str(inner),
            Self::Param(inner) => write!(f, "{{{inner}}}"),
            Self::Group(inner) => write!(f, "({inner})"),
            Self::CatchAll(inner) => write!(f, "{{*{inner}}}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_segment() {
        let seg = Segment::new("dashboard");
        assert!(seg.is_static());
        assert_eq!(seg.as_static(), Some(&"dashboard"));
    }

    #[test]
    fn param_segment() {
        let seg = Segment::new("{id}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"id"));
    }

    #[test]
    fn param_with_underscore() {
        let seg = Segment::new("{user_id}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"user_id"));
    }

    #[test]
    fn catch_all_segment() {
        let seg = Segment::new("{*rest}");
        assert!(matches!(seg, Segment::CatchAll("rest")));
    }

    #[test]
    fn group_segment() {
        let seg = Segment::new("(auth)");
        assert!(seg.is_group());
        assert_eq!(seg.as_group(), Some(&"auth"));
    }

    #[test]
    fn display_roundtrip() {
        for input in ["dashboard", "{id}", "{*rest}", "(auth)"] {
            assert_eq!(Segment::new(input).to_string(), input);
        }
    }

    #[test]
    #[should_panic(expected = "missing closing `}`")]
    fn param_missing_close() {
        Segment::new("{id");
    }

    #[test]
    #[should_panic(expected = "missing closing `)`")]
    fn group_missing_close() {
        Segment::new("(auth");
    }

    #[test]
    #[should_panic(expected = "segment must not be empty")]
    fn empty_segment() {
        Segment::new("");
    }

    #[test]
    #[should_panic(expected = "unexpected brackets")]
    fn static_with_braces() {
        Segment::new("foo{bar}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn param_empty_name() {
        Segment::new("{}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn group_empty_name() {
        Segment::new("()");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn catch_all_empty_name() {
        Segment::new("{*}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn param_invalid_start() {
        Segment::new("{0id}");
    }

    #[test]
    #[should_panic(expected = "contains invalid character")]
    fn param_invalid_char() {
        Segment::new("{id-name}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn group_invalid_start() {
        Segment::new("(0auth)");
    }

    #[test]
    #[should_panic(expected = "contains invalid character")]
    fn group_invalid_char() {
        Segment::new("(my-group)");
    }

    #[test]
    fn underscore_leading_ident() {
        let seg = Segment::new("{_private}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"_private"));
    }
}
