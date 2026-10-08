use http::{header::ACCEPT_LANGUAGE, request::Parts};
use topcoat_core::context::{Cx, app_context, memoize_cache, request_context};

use crate::{Locale, SupportedLocales};

/// Returns the locales the request accepts, most preferred first.
///
/// They are read from the `Accept-Language` header and
/// [canonicalized](Locale::canonicalize). Entries that fail to parse, the `*`
/// wildcard, and entries with a weight of zero are skipped. The result is
/// computed once per request.
///
/// # Panics
///
/// Panics if called outside a router request (no request [`Parts`] in
/// context).
#[must_use]
#[track_caller]
pub fn request_locales(cx: &Cx) -> &[Locale] {
    memoize_cache(cx).memoize(cx, (), (), |cx, ()| {
        let headers = request_context::<Parts>(&cx)
            .headers
            .get_all(ACCEPT_LANGUAGE);
        parse_accept_language(headers.iter().filter_map(|value| value.to_str().ok()))
    })
}

/// Returns the locale for the request: the supported locale that best matches
/// [`request_locales`], or the fallback.
///
/// See [`SupportedLocales::negotiate`] for how locales are matched. The
/// result is computed once per request.
///
/// # Panics
///
/// Panics if called outside a router request, or if no [`SupportedLocales`]
/// were registered on the router.
#[must_use]
#[track_caller]
pub fn locale(cx: &Cx) -> &Locale {
    memoize_cache(cx).memoize(cx, (), (), |cx, ()| {
        app_context::<SupportedLocales>(&cx)
            .negotiate(request_locales(&cx))
            .clone()
    })
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
    use topcoat_router::{
        Body, Method, Methods, Path, Route, RouteFuture, RouteId, Router, request::Request,
        response::Response,
    };

    use super::*;
    use crate::{RouterBuilderL10nExt, locale};

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

    struct CurrentLocale;

    impl Route for CurrentLocale {
        fn id(&self) -> RouteId {
            static ID: std::sync::LazyLock<RouteId> = std::sync::LazyLock::new(RouteId::new);
            *ID
        }

        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::ROOT
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move { Ok(Response::new(Body::from(locale(cx).to_string()))) })
        }
    }

    async fn locale_for(accept_language: Option<&str>) -> String {
        let router = Router::builder()
            .route(CurrentLocale)
            .supported_locales([locale!("en"), locale!("de")])
            .build();
        let mut request = Request::builder().uri("/");
        if let Some(value) = accept_language {
            request = request.header(ACCEPT_LANGUAGE, value);
        }
        let request = request.body(Body::empty()).expect("request should build");

        let response = router.handle(request).await;

        let body = topcoat_router::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        String::from_utf8(body.to_vec()).expect("body should be text")
    }

    #[tokio::test]
    async fn locale_negotiates_the_accept_language_header() {
        assert_eq!(locale_for(Some("fr, de-AT;q=0.8")).await, "de");
    }

    #[tokio::test]
    async fn locale_falls_back_without_a_header() {
        assert_eq!(locale_for(None).await, "en");
    }
}
