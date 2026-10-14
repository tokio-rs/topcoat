//! Stores the browser's light or dark theme preference in a cookie.

use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies, SameSite, UnparsedCookieStore, cookie_store, cookies, time::Duration},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn class(self) -> Option<&'static str> {
        match self {
            Self::Light => None,
            Self::Dark => Some("dark"),
        }
    }

    /// Sets the native control and scrollbar colors to match the theme.
    pub fn color_scheme(self) -> &'static str {
        match self {
            Self::Light => "color-scheme: light",
            Self::Dark => "color-scheme: dark",
        }
    }

    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

pub fn theme(cx: &Cx) -> Theme {
    store(cx).parse_or_default().get()
}

pub fn toggle_theme(cx: &Cx) -> Result<Theme> {
    let theme = store(cx)
        .parse_or_default()
        .update(|theme| *theme = theme.toggled())
        .commit()?;
    Ok(theme)
}

fn store(cx: &Cx) -> UnparsedCookieStore<Theme, impl Cookies> {
    let jar = cookies(cx)
        .default_path("/")
        .default_http_only(true)
        .default_same_site(SameSite::Lax)
        .default_max_age(Duration::days(365));
    cookie_store(jar, "theme")
}
