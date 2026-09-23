/// Builds a [`Cookie`](crate::Cookie) with optional attributes, in a syntax
/// that mirrors the [`Set-Cookie`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie)
/// header.
///
/// The name and value come first as `name = value`, followed by any number of
/// `;`-separated attributes. Flag attributes (`Secure`, `HttpOnly`) stand
/// alone; valued attributes take `Attribute = value`.
///
/// | Attribute  | Form                       | Maps to        |
/// |------------|----------------------------|----------------|
/// | `Secure`   | flag, or `Secure = bool`   | `Secure`       |
/// | `HttpOnly` | flag, or `HttpOnly = bool` | `HttpOnly`     |
/// | `Path`     | `Path = expr`              | `Path=...`     |
/// | `Domain`   | `Domain = expr`            | `Domain=...`   |
/// | `SameSite` | `SameSite = Lax`           | `SameSite=Lax` |
/// | `MaxAge`   | `MaxAge = duration`        | `Max-Age=...`  |
/// | `Expires`  | `Expires = time`           | `Expires=...`  |
///
/// The name is a single token (a string literal, an identifier, or a
/// parenthesized expression); the value and valued attributes are expressions.
/// `SameSite` accepts the bare variant names `Lax`, `Strict`, and `None` as
/// sugar, or any expression (e.g. a `SameSite`-typed variable).
///
/// The flag attributes also accept a boolean expression, such as
/// `Secure = is_prod`, to set them conditionally; a `false` value omits the
/// attribute entirely.
///
/// # Examples
///
/// ```rust
/// use topcoat_cookie::{Cookie, SameSite, cookie, time::Duration};
///
/// let plain: Cookie = cookie!("theme" = "dark");
///
/// let session: Cookie = cookie! {
///     "session" = "abc123";
///     Path = "/";
///     Secure;
///     HttpOnly;
///     SameSite = Lax;
///     MaxAge = Duration::hours(1)
/// };
///
/// assert_eq!(session.name(), "session");
/// assert_eq!(session.same_site(), Some(SameSite::Lax));
/// assert_eq!(session.secure(), Some(true));
/// ```
#[macro_export]
macro_rules! cookie {
    ($name:tt = $value:expr $(; $($attr:tt)*)?) => {{
        #[allow(unused_mut)]
        let mut cookie = $crate::Cookie::new($name, $value);
        $( $crate::__cookie_attrs!(cookie, $($attr)*); )?
        cookie
    }};
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

    // Flag attributes, conditional form: `Secure = <bool>`. A `false` value
    // leaves the attribute off entirely. Listed first so the `=` is matched
    // here rather than falling through to the bare-flag rule.
    ($cookie:ident, Secure = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_secure($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, HttpOnly = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_http_only($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // Flag attributes, bare form: always on.
    ($cookie:ident, Secure $(; $($rest:tt)*)?) => {
        $cookie.set_secure(true);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, HttpOnly $(; $($rest:tt)*)?) => {
        $cookie.set_http_only(true);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // Valued attributes.
    ($cookie:ident, Path = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_path($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Domain = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_domain($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, MaxAge = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_max_age($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
    ($cookie:ident, Expires = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_expires($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };

    // `SameSite` accepts a bare variant name (`Lax`, `Strict`, `None`) as sugar,
    // or any expression. The variant rules come first so a bare name is taken as
    // the sugar rather than an (out-of-scope) path expression.
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
    ($cookie:ident, SameSite = $value:expr $(; $($rest:tt)*)?) => {
        $cookie.set_same_site($value);
        $( $crate::__cookie_attrs!($cookie, $($rest)*); )?
    };
}

#[cfg(test)]
mod tests {
    use crate::{Cookie, SameSite, time::Duration};

    #[test]
    fn name_value_only() {
        let cookie: Cookie = cookie!("theme" = "dark");
        assert_eq!(cookie.name(), "theme");
        assert_eq!(cookie.value(), "dark");
        assert_eq!(cookie.secure(), None);
    }

    #[test]
    fn flags_and_values() {
        let cookie: Cookie = cookie! {
            "session" = "abc";
            Path = "/";
            Domain = "example.com";
            Secure;
            HttpOnly;
            SameSite = Strict;
            MaxAge = Duration::hours(2)
        };

        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.max_age(), Some(Duration::hours(2)));
    }

    #[test]
    fn conditional_flag() {
        let is_prod = false;
        let cookie: Cookie = cookie!("a" = "b"; Secure = is_prod; HttpOnly = true);
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(cookie.http_only(), Some(true));
    }

    #[test]
    fn trailing_semicolon_allowed() {
        let cookie: Cookie = cookie!("a" = "b"; Secure;);
        assert_eq!(cookie.secure(), Some(true));
    }

    #[test]
    fn expression_name_and_value() {
        let key = "id";
        let cookie: Cookie = cookie!(key = format!("{}", 42));
        assert_eq!(cookie.name(), "id");
        assert_eq!(cookie.value(), "42");
    }

    #[test]
    fn same_site_variant_and_expression() {
        let sugar: Cookie = cookie!("a" = "b"; SameSite = Lax);
        assert_eq!(sugar.same_site(), Some(SameSite::Lax));

        let chosen = SameSite::Strict;
        let from_expr: Cookie = cookie!("a" = "b"; SameSite = chosen);
        assert_eq!(from_expr.same_site(), Some(SameSite::Strict));
    }
}
