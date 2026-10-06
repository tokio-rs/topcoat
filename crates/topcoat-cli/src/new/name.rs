use std::{fmt, path::Path};

/// Names Cargo rejects for new packages: Rust keywords, the standard library crates,
/// and names with special meaning to Cargo.
const RESERVED: &[&str] = &[
    "abstract",
    "alloc",
    "as",
    "async",
    "await",
    "become",
    "box",
    "break",
    "const",
    "continue",
    "core",
    "crate",
    "deps",
    "do",
    "dyn",
    "else",
    "enum",
    "examples",
    "extern",
    "false",
    "final",
    "fn",
    "for",
    "gen",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "macro",
    "match",
    "mod",
    "move",
    "mut",
    "override",
    "priv",
    "proc-macro",
    "proc_macro",
    "pub",
    "ref",
    "return",
    "self",
    "static",
    "std",
    "struct",
    "super",
    "test",
    "trait",
    "true",
    "try",
    "type",
    "typeof",
    "unsafe",
    "unsized",
    "use",
    "virtual",
    "where",
    "while",
    "yield",
];

/// A valid name for a new Cargo package. It contains only ASCII letters, digits, `-`,
/// and `_`, so it can be written into source files without escaping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageName(String);

impl PackageName {
    /// Validates `name` as a package name.
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is empty, starts with a digit, contains characters
    /// other than ASCII letters, digits, `-`, and `_`, or is reserved.
    pub fn new(name: &str) -> Result<Self, String> {
        let valid_chars = name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if name.is_empty() {
            Err("the package name cannot be empty".to_string())
        } else if !valid_chars {
            Err(format!(
                "invalid package name `{name}`: use ASCII letters, digits, `-`, and `_`; \
                 pass --name to choose a different name than the directory"
            ))
        } else if name.starts_with(|c: char| c.is_ascii_digit()) {
            Err(format!(
                "invalid package name `{name}`: it cannot start with a digit; \
                 pass --name to choose a different name than the directory"
            ))
        } else if RESERVED.contains(&name) {
            Err(format!(
                "invalid package name `{name}`: it is reserved; \
                 pass --name to choose a different name than the directory"
            ))
        } else {
            Ok(Self(name.to_string()))
        }
    }

    /// Validates the last component of `path` as a package name.
    ///
    /// # Errors
    ///
    /// Returns an error if `path` has no last component that is valid UTF-8, or if that
    /// component is not a valid package name.
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                format!(
                    "cannot derive a package name from {}; pass --name",
                    path.display()
                )
            })?;
        Self::new(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        for name in ["my-app", "my_app", "app2", "App"] {
            assert!(PackageName::new(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_names_cargo_rejects() {
        for name in [
            "", "2app", "my app", "my.app", "app/x", "café", "fn", "test", "std",
        ] {
            assert!(PackageName::new(name).is_err(), "{name}");
        }
    }
}
