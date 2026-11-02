use topcoat::cookie::{Cookie, SameSite, cookie, time::Duration};

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
fn expressions_can_name_a_variable_called_like_the_builder() {
    let __topcoat_cookie = "/admin";
    let cookie: Cookie = cookie!(a = "b"; Path = (__topcoat_cookie));
    assert_eq!(cookie.path(), Some("/admin"));
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
        Max-Age = (Duration::hours(2));
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
fn same_site_variant_and_expression() {
    let sugar: Cookie = cookie!(a = "b"; SameSite = Lax);
    assert_eq!(sugar.same_site(), Some(SameSite::Lax));

    let chosen = SameSite::Strict;
    let from_expr: Cookie = cookie!(a = "b"; SameSite = (chosen));
    assert_eq!(from_expr.same_site(), Some(SameSite::Strict));
}
