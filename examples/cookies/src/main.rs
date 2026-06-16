use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies, Key, cookie, signed_cookies},
    router::{Router, page},
    view::view,
};

#[tokio::main]
async fn main() {
    // The Key signs cookies so the client can't forge the counter. Generate it
    // once at startup and share it across requests as app state. A real app
    // would load a persisted key instead of generating a fresh one each boot.
    topcoat::start(Router::new().discover().app_state(Key::generate()))
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

// Reads the visit count from the request. Returns 0 when the cookie is missing
// or its signature fails to verify.
fn visits(cx: &Cx) -> u32 {
    cookies(cx)
        .get("visits")
        .and_then(|c| c.value().parse().ok())
        .unwrap_or(0)
}

// Queues a Set-Cookie with the new count; the router writes it to the response
// automatically. Path and HttpOnly come from the jar defaults.
fn set_visits(cx: &Cx, count: u32) {
    cookies(cx).add(cookie!("visits" = count.to_string()));
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let count = visits(cx) + 1;
    set_visits(cx, count);

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
            </head>
            <body>
                <p>"You have visited this page " (count) " times."</p>
            </body>
        </html>
    }
}
