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
    // The key signs cookies so the client cannot forge the visit counter.
    // A real application should load a persistent key instead of generating
    // a new one whenever the server starts.
    //
    // By default, the application is available at http://127.0.0.1:3000.
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

// Build the signed cookie jar used by this application.
//
// Every cookie created through this helper:
// - is available across the complete application;
// - cannot be accessed through browser JavaScript;
// - is sent only in a secure context.
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

// Read the typed "visits" cookie.
//
// If the cookie is missing or its signature is invalid, start from the
// default Visits value, which contains zero visits.
fn visits(cx: &Cx) -> CookieStore<Visits, impl Cookies> {
    cookie_store(cookies(cx), "visits").parse_or_default()
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    // Increment the typed cookie value and add the updated cookie
    // to the HTTP response through a Set-Cookie header.
    let visits = visits(cx).update(Visits::increment).commit()?;

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Cookie"</title>
                topcoat::dev::script()
            </head>
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
