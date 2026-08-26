use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    cookie::{CookieStore, Cookies, Key, RouterBuilderCookieExt, cookie_store, signed_cookies},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    // The key signs cookies so the client cannot forge the visit counter.
    // A real application would load a persistent key instead of generating
    // a new one on every start.
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

// The jar every handler goes through, so its attribute defaults are set once.
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

// A missing or tampered cookie falls back to the default `Visits`.
fn visits(cx: &Cx) -> CookieStore<Visits, impl Cookies> {
    cookie_store(cookies(cx), "visits").parse_or_default()
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    // `commit` queues the updated value as a Set-Cookie response header.
    let visits = visits(cx).update(Visits::increment).commit()?;

    Ok(view! {
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
    })
}
