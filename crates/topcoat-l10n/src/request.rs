use http::{header::ACCEPT_LANGUAGE, request::Parts};
use topcoat_core::context::{Cx, memoize_cache, try_context};

use crate::Locale;

/// Reads the request's `Accept-Language` header and lists locales in preference order.
/// Returns an empty slice outside a router request.
///
/// [Canonicalizes](Locale::canonicalize) the locale names and skips invalid entries,
/// `*` and entries with zero weight. Caches the result for the request.
#[must_use]
pub fn accept_language(cx: &Cx) -> &[Locale] {
    memoize_cache(cx).memoize(cx, (), (), |cx, ()| {
        let Some(parts) = try_context::<Parts>(&cx) else {
            return Vec::new();
        };
        let headers = parts.headers.get_all(ACCEPT_LANGUAGE);
        parse_accept_language(headers.iter().filter_map(|value| value.to_str().ok()))
    })
}

/// Parses header values into locales sorted by preference. Equal weights preserve the
/// input order.
fn parse_accept_language<'a>(values: impl Iterator<Item = &'a str>) -> Vec<Locale> {
    let mut weighted: Vec<_> = values
        .flat_map(|value| value.split(','))
        .filter_map(parse_entry)
        .collect();
    weighted.sort_by_key(|(weight, _)| std::cmp::Reverse(*weight));
    weighted.into_iter().map(|(_, locale)| locale).collect()
}

/// Parses a `tag;q=weight` entry into a locale and a weight measured in thousandths.
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

/// Converts a weight from `0` to `1`, with at most three decimal places, into
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
    use crate::{locale, try_locale};

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

    /// Creates a request context with the given `Accept-Language` header.
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
    fn the_header_does_not_choose_a_locale() {
        assert_eq!(try_locale(&request("de")), None);
    }
}
