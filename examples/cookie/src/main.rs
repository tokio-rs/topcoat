use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    cookie::{CookieStore, Cookies, Key, RouterBuilderCookieExt, cookie_store, signed_cookies},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

#[tokio::main]
async fn main() {
    // The Key signs cookies so the client can't forge the counter. Generate it
    // once at startup and share it across requests as app context. A real app
    // would load a persisted key instead of generating a fresh one each boot.
    topcoat::start(
        Router::builder()
            .discover()
            .cookies()
            .app_context(Key::generate())
            .build(),
    )
    .await
    .unwrap();
}

// The application cookie jar: signed with the registered Key, with our defaults
// baked in. Handlers use this instead of the bare topcoat::cookie::cookies so
// every cookie gets the same attributes and we can tighten them in one place.
fn cookies(cx: &Cx) -> impl Cookies {
    signed_cookies(cx)
        .default_path("/")
        .default_http_only(true)
        .override_secure(true)
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
struct Visits(u32);

impl Visits {
    fn increment(&mut self) {
        self.0 += 1;
    }
}

// Builds a typed store over the "visits" cookie, reading the current value from
// the request and falling back to Visits::default() (zero) when it's missing.
// Like the cookies jar above, wrapping it in a helper keeps the name and jar
// consistent across handlers.
fn visits(cx: &Cx) -> CookieStore<Visits, impl Cookies> {
    cookie_store(cookies(cx), "visits").parse_or_default()
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    // Increment the visit counter and queue the appropriate `Set-Cookie`
    // header in the HTTP response.
    let visits = visits(cx).update(Visits::increment).commit()?;

    view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body>
                <p>
                    "You have visited this page "
                    (visits.0)
                    " times."
                </p>
            </body>
        </html>
    }
}
