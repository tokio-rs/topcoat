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

// A handle to the request's visit count, tied to the request context so it can
// write the cookie back when incremented.
struct Visits<'cx> {
    cx: &'cx Cx,
    count: u32,
}

impl<'cx> Visits<'cx> {
    // Reads the count from the request. Starts at 0 when the cookie is missing
    // or its signature fails to verify.
    fn of(cx: &'cx Cx) -> Self {
        let count = cookies(cx)
            .get("visits")
            .and_then(|c| c.value().parse().ok())
            .unwrap_or(0);

        Self { cx, count }
    }

    // Bumps the count and queues a Set-Cookie; the router writes it to the
    // response automatically.
    fn increment(&mut self) {
        self.count += 1;
        cookies(self.cx).add(cookie!("visits" = self.count.to_string()));
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let mut visits = Visits::of(cx);
    visits.increment();

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
            </head>
            <body>
                <p>"You have visited this page " (visits.count) " times."</p>
            </body>
        </html>
    }
}
