Builds a [`Cookie`] using the syntax of the [`Set-Cookie`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie) header.

Start with `name = value`, then separate attributes with `;`. Write a flag such as `Secure` on its own. Other attributes use `Attribute = value`.

Literals are written as they are, as in `Path = "/"` or `Max-Age = 3600`. Rust expressions are wrapped in parentheses, as in `session = (token)`.

| Attribute     | Form                                     | Maps to        |
|---------------|------------------------------------------|----------------|
| `Path`        | `Path = "/"`, `Path = (path)`            | `Path=...`     |
| `Domain`      | `Domain = "example.com"`, `Domain = (d)` | `Domain=...`   |
| `Max-Age`     | `Max-Age = 3600`, `Max-Age = (duration)` | `Max-Age=...`  |
| `Expires`     | `Expires = (time)`                       | `Expires=...`  |
| `Secure`      | flag, or `Secure = (bool)`               | `Secure`       |
| `HttpOnly`    | flag, or `HttpOnly = (bool)`             | `HttpOnly`     |
| `Partitioned` | flag, or `Partitioned = (bool)`          | `Partitioned`  |
| `SameSite`    | `SameSite = Lax`, `SameSite = (value)`   | `SameSite=Lax` |

The cookie name is written as it appears in the header, such as `theme` or `__Host-session`. Use a string literal for names that Rust cannot read as tokens, or a parenthesized expression for a name computed at runtime. The value is a literal or a parenthesized expression.

An integer `Max-Age` counts seconds, as in the header. `SameSite` accepts the variant names `Lax`, `Strict`, and `None`. A flag set to `(false)` leaves the attribute off.

```rust
use topcoat::cookie::{Cookie, SameSite, cookie, time::Duration};

let plain: Cookie = cookie!(theme = "dark");

let token = String::from("abc123");
let session: Cookie = cookie! {
    __Host-session = (token);
    Path = "/";
    Max-Age = 3600;
    Secure;
    HttpOnly;
    SameSite = Lax;
};

assert_eq!(session.name(), "__Host-session");
assert_eq!(session.max_age(), Some(Duration::hours(1)));
assert_eq!(session.same_site(), Some(SameSite::Lax));
assert_eq!(session.secure(), Some(true));
```

[`Cookie`]: struct.Cookie.html
