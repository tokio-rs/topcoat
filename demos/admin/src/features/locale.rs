//! Chooses the app's language from a cookie or the browser's preferred languages.

use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies, SameSite, UnparsedCookieStore, cookie_store, cookies, time::Duration},
    l10n::{Locale, accept_language, locale, negotiate},
};

/// The languages the app is translated into. The first one is the default.
pub static LOCALES: [Locale; 2] = [locale!("en"), locale!("de")];

/// Returns the language chosen with [`choose_locale`]. Without a choice, returns
/// the best match for the browser's languages.
pub fn preferred_locale(cx: &Cx) -> &'static Locale {
    let chosen = store(cx)
        .parse()
        .ok()
        .flatten()
        .and_then(|store| store.read().parse::<Locale>().ok());
    negotiate(chosen.iter().chain(accept_language(cx)), &LOCALES).unwrap_or(&LOCALES[0])
}

/// Remembers `locale` as this browser's language.
pub fn choose_locale(cx: &Cx, locale: &Locale) -> Result<()> {
    store(cx).set(locale.to_string()).commit()?;
    Ok(())
}

fn store(cx: &Cx) -> UnparsedCookieStore<String, impl Cookies> {
    let jar = cookies(cx)
        .default_path("/")
        .default_http_only(true)
        .default_same_site(SameSite::Lax)
        .default_max_age(Duration::days(365));
    cookie_store(jar, "locale")
}
