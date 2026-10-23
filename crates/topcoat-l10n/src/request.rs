use http::{header::ACCEPT_LANGUAGE, request::Parts};
use topcoat_core::context::{Cx, memoize_cache, try_request_context};

use crate::Locale;

/// Returns the locales listed in the request's `Accept-Language` header, most
/// preferred first, or none outside a router request.
///
/// The locales are [canonicalized](Locale::canonicalize). Entries that fail to
/// parse, the `*` wildcard, and entries with a weight of zero are skipped. The
/// result is computed once per request.
#[must_use]
pub fn accept_language(cx: &Cx) -> &[Locale] {
    memoize_cache(cx).memoize(cx, (), (), |cx, ()| {
        let Some(parts) = try_request_context::<Parts>(&cx) else {
            return Vec::new();
        };
        let headers = parts.headers.get_all(ACCEPT_LANGUAGE);
        parse_accept_language(headers.iter().filter_map(|value| value.to_str().ok()))
    })
}

/// Returns the locale of the current scope, or `None` when it has none.
///
/// The locale is the [`Locale`] registered on the request context, such as by
/// a layer with [`Cx::with`].
#[must_use]
pub fn try_locale(cx: &Cx) -> Option<&Locale> {
    try_request_context(cx)
}

/// Returns the locale of the current scope.
///
/// See [`try_locale`] for where it is registered.
///
/// # Panics
///
/// Panics if no [`Locale`] is registered.
#[must_use]
#[track_caller]
pub fn locale(cx: &Cx) -> &Locale {
    let Some(locale) = try_locale(cx) else {
        panic!("no locale: register a `Locale` on the request context, such as in a layer");
    };
    locale
}

/// Parses `Accept-Language` header values into locales, most preferred
/// first. Entries with the same weight keep their order.
fn parse_accept_language<'a>(values: impl Iterator<Item = &'a str>) -> Vec<Locale> {
    let mut weighted: Vec<_> = values
        .flat_map(|value| value.split(','))
        .filter_map(parse_entry)
        .collect();
    weighted.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
    weighted.into_iter().map(|(_, locale)| locale).collect()
}

/// Parses one `tag;q=weight` entry into its weight in thousandths and its
/// locale.
fn parse_entry(entry: &str) -> Option<(u16, Locale)> {
    let mut parts = entry.split(';');
    let tag = parts.next()?.trim();
    let mut weight = 1000;
    for parameter in parts {
        let (name, value) = parameter.split_once('=')?;
        if name.trim().eq_ignore_ascii_case("q") {
            weight = parse_weight(value.trim())?;
        }
    }
    if weight == 0 || tag == "*" {
        return None;
    }
    let locale = tag.parse::<Locale>().ok()?.canonicalize();
    Some((weight, locale))
}

/// Parses a weight between `0` and `1` with up to three decimals into
/// thousandths.
fn parse_weight(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whole = match whole {
        "0" => 0,
        "1" => 1000,
        _ => return None,
    };
    let fraction = fraction
        .bytes()
        .zip([100, 10, 1])
        .map(|(digit, scale)| u16::from(digit - b'0') * scale)
        .sum::<u16>();
    let weight = whole + fraction;
    (weight <= 1000).then_some(weight)
}

#[cfg(test)]
mod tests {
    use http::Request;

    use super::*;
    use crate::locale;

    fn parse(value: &str) -> Vec<Locale> {
        parse_accept_language([value].into_iter())
    }

    #[test]
    fn orders_by_weight_and_keeps_header_order_for_ties() {
        assert_eq!(
            parse("de;q=0.5, fr-CH, en;q=0.8, fr"),
            [
                locale!("fr-CH"),
                locale!("fr"),
                locale!("en"),
                locale!("de")
            ]
        );
    }

    #[test]
    fn combines_several_header_values() {
        let locales = parse_accept_language(["de;q=0.5", "en"].into_iter());

        assert_eq!(locales, [locale!("en"), locale!("de")]);
    }

    #[test]
    fn skips_unacceptable_and_malformed_entries() {
        assert_eq!(
            parse("*, de;q=0, e1, , fr;q=2, it;q=abc, es;q=0.5;level=1, nl"),
            [locale!("nl"), locale!("es")]
        );
    }

    #[test]
    fn tolerates_whitespace_and_case() {
        assert_eq!(
            parse(" EN-us ; Q=0.9 ,de"),
            [locale!("de"), locale!("en-US")]
        );
    }

    #[test]
    fn canonicalizes_deprecated_subtags() {
        assert_eq!(parse("iw"), [locale!("he")]);
    }

    #[test]
    fn parses_weights_in_thousandths() {
        assert_eq!(parse_weight("1"), Some(1000));
        assert_eq!(parse_weight("1.000"), Some(1000));
        assert_eq!(parse_weight("0.5"), Some(500));
        assert_eq!(parse_weight("0.125"), Some(125));
        assert_eq!(parse_weight("0"), Some(0));
        assert_eq!(parse_weight("1.5"), None);
        assert_eq!(parse_weight("0.1234"), None);
        assert_eq!(parse_weight(".5"), None);
    }

    /// Returns a context for a request with the `Accept-Language` header
    /// `header`.
    fn request(header: &str) -> Cx {
        let (parts, ()) = Request::builder()
            .header(ACCEPT_LANGUAGE, header)
            .body(())
            .expect("request should build")
            .into_parts();
        Cx::default().with(parts)
    }

    #[test]
    fn reads_the_header_of_the_request() {
        assert_eq!(
            accept_language(&request("de;q=0.5, en")),
            [locale!("en"), locale!("de")]
        );
    }

    #[test]
    fn there_is_no_header_outside_a_request() {
        assert_eq!(accept_language(&Cx::default()), []);
    }

    #[test]
    fn the_locale_is_the_one_in_the_scope() {
        let cx = request("de").with(locale!("fr"));
        assert_eq!(locale(&cx), &locale!("fr"));
    }

    #[test]
    fn the_header_does_not_choose_a_locale() {
        assert_eq!(try_locale(&request("de")), None);
    }

    #[test]
    #[should_panic(expected = "no locale")]
    fn locale_panics_without_a_locale() {
        let _ = locale(&Cx::default());
    }
}
