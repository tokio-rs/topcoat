use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, Span};
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

/// The string argument of `include!` and `iconify_icon!`: a selection from a
/// staged icon set in one of the forms `"mdi"`, `"mdi:*"`, or `"mdi:delete"`.
pub struct Selection {
    /// The written literal, kept for error spans.
    pub lit: LitStr,
    /// The set's prefix, before the `:`.
    pub prefix: String,
    /// What is selected from the set.
    pub selected: Selected,
}

impl Selection {
    /// The const name for an icon of the selected set: kebab-case becomes
    /// `SCREAMING_SNAKE_CASE` (`trash-2` -> `TRASH_2`), and a leading digit
    /// gains a `_` prefix (`2fa` -> `_2FA`). Iconify names never contain
    /// underscores, so distinct icons cannot collide.
    #[must_use]
    pub fn const_ident(&self, icon: &str) -> Ident {
        Ident::new(
            &guard_leading_digit(icon.to_shouty_snake_case()),
            self.lit.span(),
        )
    }

    /// The module name for the selected set: the same rule as
    /// [`const_ident`](Self::const_ident), in lowercase (`simple-icons` ->
    /// `simple_icons`).
    #[must_use]
    pub fn module_ident(&self) -> Ident {
        let name = guard_leading_digit(self.prefix.to_snake_case());
        // A prefix like `box` turns into a keyword when it becomes a module
        // name.
        if syn::parse_str::<Ident>(&name).is_ok() {
            Ident::new(&name, self.lit.span())
        } else {
            Ident::new_raw(&name, self.lit.span())
        }
    }

    /// The span of the written literal, anchoring errors and generated items
    /// to the invocation.
    #[must_use]
    pub fn span(&self) -> Span {
        self.lit.span()
    }
}

impl Parse for Selection {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        let value = lit.value();
        let (prefix, selected) = match value.split_once(':') {
            None => (value.as_str(), Selected::Set),
            Some((prefix, "*")) => (prefix, Selected::Glob),
            Some((prefix, icon)) => (prefix, Selected::Icon(icon.to_owned())),
        };

        validate_name(prefix, "icon set prefix", &lit)?;
        if let Selected::Icon(icon) = &selected {
            validate_name(icon, "icon name", &lit)?;
        }

        Ok(Self {
            prefix: prefix.to_owned(),
            lit,
            selected,
        })
    }
}

/// Prefixes `name` with a `_` when it starts with a digit, which idents must
/// not.
fn guard_leading_digit(name: String) -> String {
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// What a [`Selection`] selects from its icon set.
pub enum Selected {
    /// `"mdi"`: every icon, wrapped in a module named after the set.
    Set,
    /// `"mdi:*"`: every icon, inlined into the current scope.
    Glob,
    /// `"mdi:delete"`: a single icon.
    Icon(String),
}

/// Checks that a written set prefix or icon name sticks to Iconify's
/// character set.
fn validate_name(name: &str, what: &str, lit: &LitStr) -> syn::Result<()> {
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    if name.is_empty() || !name.chars().all(valid) {
        return Err(syn::Error::new(
            lit.span(),
            format!("invalid {what} `{name}`: expected a lowercase name of `a-z`, `0-9`, and `-`"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(source: &str) -> Selection {
        syn::parse_str(&format!("\"{source}\"")).unwrap()
    }

    #[test]
    fn const_names_are_screaming_snake_case() {
        let name = |icon| selection("demo").const_ident(icon).to_string();

        assert_eq!(name("delete"), "DELETE");
        assert_eq!(name("trash-2"), "TRASH_2");
    }

    #[test]
    fn leading_digits_gain_an_underscore() {
        let name = |icon| selection("demo").const_ident(icon).to_string();

        assert_eq!(name("123"), "_123");
        assert_eq!(name("24-hours"), "_24_HOURS");
        assert_eq!(name("2fa"), "_2FA");
    }

    #[test]
    fn module_names_are_snake_case() {
        assert_eq!(selection("mdi").module_ident().to_string(), "mdi");
        assert_eq!(
            selection("simple-icons").module_ident().to_string(),
            "simple_icons"
        );
    }

    #[test]
    fn keyword_module_names_become_raw() {
        assert_eq!(selection("box").module_ident().to_string(), "r#box");
    }

    #[test]
    fn selections_parse_into_their_three_forms() {
        assert!(matches!(selection("mdi").selected, Selected::Set));
        assert!(matches!(selection("mdi:*").selected, Selected::Glob));
        assert!(
            matches!(&selection("mdi:trash-2").selected, Selected::Icon(icon) if icon == "trash-2")
        );
    }

    #[test]
    fn invalid_names_are_rejected() {
        let error = |source: &str| {
            syn::parse_str::<Selection>(&format!("\"{source}\""))
                .err()
                .unwrap()
                .to_string()
        };

        assert!(error("").contains("invalid icon set prefix"));
        assert!(error("Mdi:delete").contains("invalid icon set prefix"));
        assert!(error("mdi:").contains("invalid icon name"));
        assert!(error("mdi:Delete").contains("invalid icon name"));
        assert!(error("mdi:a:b").contains("invalid icon name"));
    }
}
