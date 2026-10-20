use http::{header::ACCEPT_LANGUAGE, request::Parts};
use topcoat_core::context::{Cx, memoize_cache, try_app_context, try_request_context};

use crate::{Locale, SupportedLocales};

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

/// Returns the locales supported in the current scope, or `None` when none are
/// registered.
///
/// Locales registered on the request context, such as by a layer with
/// [`Cx::with`], take precedence over those registered on the app context,
/// such as with
/// [`supported_locales`](crate::RouterBuilderL10nExt::supported_locales).
#[must_use]
pub fn try_supported_locales(cx: &Cx) -> Option<&SupportedLocales> {
    try_request_context(cx).or_else(|| try_app_context(cx))
}

/// Returns the locales supported in the current scope.
///
/// See [`try_supported_locales`] for where they are registered.
///
/// # Panics
///
/// Panics if no [`SupportedLocales`] are registered.
#[must_use]
#[track_caller]
pub fn supported_locales(cx: &Cx) -> &SupportedLocales {
    let Some(supported) = try_supported_locales(cx) else {
        panic!(
            "no supported locales: register `SupportedLocales` on the router or the request \
             context"
        );
    };
    supported
}

/// Returns the locale of the current scope, or `None` when it has none.
///
/// A [`Locale`] registered on the request context, such as by a layer with
/// [`Cx::with`], is the locale. Otherwise, it is the
/// [supported locale](try_supported_locales) that best matches
/// [`accept_language`], or their fallback. See [`SupportedLocales::negotiate`]
/// for how locales are matched. The result is computed once per request and
/// scope.
#[must_use]
pub fn try_locale(cx: &Cx) -> Option<&Locale> {
    if let Some(locale) = try_request_context(cx) {
        return Some(locale);
    }
    memoize_cache(cx)
        .memoize(cx, (), (), |cx, ()| {
            try_supported_locales(&cx)
                .map(|supported| supported.negotiate(accept_language(&cx)).clone())
        })
        .as_ref()
}

/// Returns the locale of the current scope.
///
/// See [`try_locale`] for how it is chosen.
///
/// # Panics
///
/// Panics if neither a [`Locale`] nor [`SupportedLocales`] are registered.
#[must_use]
#[track_caller]
pub fn locale(cx: &Cx) -> &Locale {
    let Some(locale) = try_locale(cx) else {
        panic!(
            "no locale: register a `Locale` or `SupportedLocales` on the router or the request \
             context"
        );
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

    /// A route that responds with the text returned by its function.
    struct Respond(fn(&Cx) -> String);

    impl Route for Respond {
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
            Box::pin(async move { Ok(Response::new(Body::from((self.0)(cx)))) })
        }
    }

    /// Handles a request with the `Accept-Language` header `header` on a
    /// router supporting `en` and `de` when `supported` is set.
    async fn respond(supported: bool, header: Option<&str>, respond: fn(&Cx) -> String) -> String {
        let mut router = Router::builder().route(Respond(respond));
        if supported {
            router = router.supported_locales([locale!("en"), locale!("de")]);
        }
        let router = router.build();
        let mut request = Request::builder().uri("/");
        if let Some(value) = header {
            request = request.header(ACCEPT_LANGUAGE, value);
        }
        let request = request.body(Body::empty()).expect("request should build");

        let response = router.handle(request).await;

        let body = topcoat_router::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        String::from_utf8(body.to_vec()).expect("body should be text")
    }

    fn current_locale(cx: &Cx) -> String {
        locale(cx).to_string()
    }

    #[tokio::test]
    async fn locale_negotiates_the_accept_language_header() {
        assert_eq!(respond(true, Some("fr, de-AT;q=0.8"), current_locale).await, "de");
    }

    #[tokio::test]
    async fn locale_falls_back_without_a_header() {
        assert_eq!(respond(true, None, current_locale).await, "en");
    }

    #[tokio::test]
    async fn a_locale_in_the_request_context_is_the_locale() {
        let locale = respond(true, Some("de"), |cx| {
            current_locale(&cx.with(locale!("fr")))
        });
        assert_eq!(locale.await, "fr");
    }

    #[tokio::test]
    async fn supported_locales_in_the_request_context_take_precedence() {
        // The outer scope negotiates `de` from the router's locales. The inner
        // scope supports only `fr` and `it`, so it falls back to `fr`.
        let locales = respond(true, Some("de"), |cx| {
            let scoped = cx.with(SupportedLocales::from([locale!("fr"), locale!("it")]));
            format!("{} {}", current_locale(cx), current_locale(&scoped))
        });
        assert_eq!(locales.await, "de fr");
    }

    #[tokio::test]
    async fn there_is_no_locale_without_supported_locales() {
        let locale = respond(false, Some("de"), |cx| format!("{:?}", try_locale(cx)));
        assert_eq!(locale.await, "None");
    }

    #[test]
    fn outside_a_request_the_locale_is_the_fallback() {
        let cx = Cx::default().with(SupportedLocales::from([locale!("de"), locale!("en")]));
        assert!(accept_language(&cx).is_empty());
        assert_eq!(locale(&cx), &locale!("de"));
    }

    #[test]
    #[should_panic(expected = "no locale")]
    fn locale_panics_without_supported_locales() {
        let _ = locale(&Cx::default());
    }
}
