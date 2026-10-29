/// Builds a [`Cookie`](crate::Cookie) using the syntax of the
/// [`Set-Cookie`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie) header.
///
/// Start with `name = value`, then separate attributes with `;`. Write a flag
/// such as `Secure` on its own. Other attributes use `Attribute = value`.
///
/// Literals are written as they are, as in `Path = "/"` or `Max-Age = 3600`.
/// Rust expressions are wrapped in parentheses, as in `session = (token)`.
///
/// | Attribute     | Form                                     | Maps to        |
/// |---------------|------------------------------------------|----------------|
/// | `Path`        | `Path = "/"`, `Path = (path)`            | `Path=...`     |
/// | `Domain`      | `Domain = "example.com"`, `Domain = (d)` | `Domain=...`   |
/// | `Max-Age`     | `Max-Age = 3600`, `Max-Age = (duration)` | `Max-Age=...`  |
/// | `Expires`     | `Expires = (time)`                       | `Expires=...`  |
/// | `Secure`      | flag, or `Secure = (bool)`               | `Secure`       |
/// | `HttpOnly`    | flag, or `HttpOnly = (bool)`             | `HttpOnly`     |
/// | `Partitioned` | flag, or `Partitioned = (bool)`          | `Partitioned`  |
/// | `SameSite`    | `SameSite = Lax`, `SameSite = (value)`   | `SameSite=Lax` |
///
/// The cookie name is written as it appears in the header, such as `theme` or
/// `__Host-session`. Use a string literal for names that Rust cannot read as
/// tokens, or a parenthesized expression for a name computed at runtime. The
/// value is a literal or a parenthesized expression.
///
/// An integer `Max-Age` counts seconds, as in the header. `SameSite` accepts
/// the variant names `Lax`, `Strict`, and `None`. A flag set to `(false)`
/// leaves the attribute off.
///
/// # Examples
///
/// ```rust
/// use topcoat_cookie::{Cookie, SameSite, cookie, time::Duration};
///
/// let plain: Cookie = cookie!(theme = "dark");
///
/// let token = String::from("abc123");
/// let session: Cookie = cookie! {
///     __Host-session = (token);
///     Path = "/";
///     Max-Age = 3600;
///     Secure;
///     HttpOnly;
///     SameSite = Lax;
/// };
///
/// assert_eq!(session.name(), "__Host-session");
/// assert_eq!(session.max_age(), Some(Duration::hours(1)));
/// assert_eq!(session.same_site(), Some(SameSite::Lax));
/// assert_eq!(session.secure(), Some(true));
/// ```
#[macro_export]
macro_rules! cookie {
    ($name:literal = $($rest:tt)*) => {
        $crate::__cookie!($name; $($rest)*)
    };
    (($name:expr) = $($rest:tt)*) => {
        $crate::__cookie!($name; $($rest)*)
    };
    ($first:ident $(- $part:tt)* = $($rest:tt)*) => {
        $crate::__cookie!(
            ::core::concat!(::core::stringify!($first) $(, "-", ::core::stringify!($part))*);
            $($rest)*
        )
    };
}

/// Reads the value from [`cookie!`] and builds the cookie with its attributes.
#[doc(hidden)]
#[macro_export]
macro_rules! __cookie {
    // Matched first: once `$name:expr` has parsed, a failing rule cannot
    // fall through to the next one.
    (@build $name:expr; $value:expr; $($attr:tt)*) => {{
        #[allow(unused_mut)]
        let mut cookie = $crate::Cookie::new($name, $value);
        $crate::__cookie_attrs!(cookie, $($attr)*);
        cookie
    }};
    ($name:expr; $value:literal $(; $($attr:tt)*)?) => {
        $crate::__cookie!(@build $name; $value; $($($attr)*)?)
    };
    ($name:expr; ($value:expr) $(; $($attr:tt)*)?) => {
        $crate::__cookie!(@build $name; $value; $($($attr)*)?)
    };
    ($name:expr; $($rest:tt)*) => {
        ::core::compile_error!(
            "expected a literal or a parenthesized expression as the cookie value"
        )
    };
}

/// Applies the `;`-separated attribute list from [`cookie!`] to a builder,
/// one attribute per rule, recursing on the remainder.
#[doc(hidden)]
#[macro_export]
macro_rules! __cookie_attrs {
    // Done, or a stray separator.
    ($cookie:ident $(,)?) => {};
    ($cookie:ident, ; $($rest:tt)*) => {
        $crate::__cookie_attrs!($cookie, $($rest)*);
    };

    // Flags, conditional form: `Secure = (bool)`. A `false` value leaves the
    // attribute off. Listed before the bare form so the `=` is matched here.
    ($cookie:ident, Secure = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_secure($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, HttpOnly = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_http_only($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Partitioned = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_partitioned($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // Flags, bare form: always on.
    ($cookie:ident, Secure $(; $($rest:tt)*)?) => {
        $cookie.set_secure(true);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, HttpOnly $(; $($rest:tt)*)?) => {
        $cookie.set_http_only(true);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Partitioned $(; $($rest:tt)*)?) => {
        $cookie.set_partitioned(true);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // Valued attributes, as a literal or a parenthesized expression.
    ($cookie:ident, Path = $value:literal $(; $($rest:tt)*)?) => {
        $cookie.set_path($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Path = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_path($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Domain = $value:literal $(; $($rest:tt)*)?) => {
        $cookie.set_domain($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Domain = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_domain($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    // An integer literal counts seconds, as in the header.
    ($cookie:ident, Max-Age = $seconds:literal $(; $($rest:tt)*)?) => {
        $cookie.set_max_age($crate::time::Duration::seconds($seconds));
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Max-Age = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_max_age($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Expires = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_expires($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // `SameSite` accepts a variant name (`Lax`, `Strict`, `None`) or a
    // parenthesized expression.
    ($cookie:ident, SameSite = Lax $(; $($rest:tt)*)?) => {
        $cookie.set_same_site($crate::SameSite::Lax);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, SameSite = Strict $(; $($rest:tt)*)?) => {
        $cookie.set_same_site($crate::SameSite::Strict);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, SameSite = None $(; $($rest:tt)*)?) => {
        $cookie.set_same_site($crate::SameSite::None);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, SameSite = ($value:expr) $(; $($rest:tt)*)?) => {
        $cookie.set_same_site($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    ($cookie:ident, $attr:tt $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "invalid cookie attribute starting with `",
            ::core::stringify!($attr),
            "`; see the `cookie!` documentation for the supported forms",
        ));
    };
}

#[cfg(test)]
mod tests {
    use crate::{Cookie, SameSite, time::Duration};

    #[test]
    fn name_value_only() {
        let cookie: Cookie = cookie!(theme = "dark");
        assert_eq!(cookie.name(), "theme");
        assert_eq!(cookie.value(), "dark");
        assert_eq!(cookie.secure(), None);
    }

    #[test]
    fn hyphenated_names() {
        let cookie: Cookie = cookie!(__Host - csrf - token = "x");
        assert_eq!(cookie.name(), "__Host-csrf-token");
    }

    #[test]
    fn literal_and_expression_names() {
        let quoted: Cookie = cookie!("my.cookie" = "x");
        assert_eq!(quoted.name(), "my.cookie");

        let theme = "dark";
        let computed: Cookie = cookie!((format!("{theme}-mode")) = "on");
        assert_eq!(computed.name(), "dark-mode");
    }

    #[test]
    fn a_bare_name_is_not_a_variable() {
        let theme = "unused";
        let cookie: Cookie = cookie!(theme = (theme));
        assert_eq!(cookie.name(), "theme");
        assert_eq!(cookie.value(), "unused");
    }

    #[test]
    fn flags_and_values() {
        let cookie: Cookie = cookie! {
            session = "abc";
            Path = "/";
            Domain = "example.com";
            Secure;
            HttpOnly;
            Partitioned;
            SameSite = Strict;
            Max-Age = (Duration::hours(2))
        };

        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.partitioned(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.max_age(), Some(Duration::hours(2)));
    }

    #[test]
    fn max_age_literal_counts_seconds() {
        let cookie: Cookie = cookie!(a = "b"; Max-Age = 90);
        assert_eq!(cookie.max_age(), Some(Duration::seconds(90)));
    }

    #[test]
    fn expression_values() {
        let token = String::from("abc");
        let path = "/admin";
        let cookie: Cookie = cookie!(session = (token.clone()); Path = (path));
        assert_eq!(cookie.value(), "abc");
        assert_eq!(cookie.path(), Some("/admin"));
    }

    #[test]
    fn conditional_flag() {
        let is_prod = false;
        let cookie: Cookie = cookie!(a = "b"; Secure = (is_prod); HttpOnly = (true));
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(cookie.http_only(), Some(true));
    }

    #[test]
    fn trailing_semicolon_allowed() {
        let cookie: Cookie = cookie!(a = "b"; Secure;);
        assert_eq!(cookie.secure(), Some(true));
    }

    #[test]
    fn same_site_variant_and_expression() {
        let sugar: Cookie = cookie!(a = "b"; SameSite = Lax);
        assert_eq!(sugar.same_site(), Some(SameSite::Lax));

        let chosen = SameSite::Strict;
        let from_expr: Cookie = cookie!(a = "b"; SameSite = (chosen));
        assert_eq!(from_expr.same_site(), Some(SameSite::Strict));
    }
}
