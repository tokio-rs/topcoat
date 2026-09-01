use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, Slot, error::redirect, href, layout, page},
    view::{View, component, error_boundary, suspense, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[layout("/")]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Suspense"</title>

                // Reloads the browser when the dev server rebuilds the app.
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href=(href!(quote))>"Quote"</a>
                    " | "
                    <a href=(href!(dashboard))>"Dashboard"</a>
                    " | "
                    <a href=(href!(profile))>"Profile"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}

// The shell and the fallback reach the browser right away; the quote replaces
// the fallback in place once it has rendered.
#[page("/")]
async fn quote() -> Result<impl View> {
    Ok(view! {
        <h1>"Quote of the day"</h1>
        suspense(fallback: view! { <p>"Looking one up..."</p> }, daily_quote())
    })
}

#[component]
async fn daily_quote() -> Result<impl View> {
    Ok(view! { <blockquote>(fetch_quote().await)</blockquote> })
}

// Stands in for a slow database query or upstream request.
async fn fetch_quote() -> &'static str {
    tokio::time::sleep(Duration::from_secs(2)).await;
    "Simplicity is prerequisite for reliability."
}

// Each widget streams in behind its own fallback. The visits lookup fails,
// but the boundary around it turns the error into a message in place, and
// the rest of the dashboard is unaffected.
#[page("/dashboard")]
async fn dashboard() -> Result<impl View> {
    Ok(view! {
        <h1>"Dashboard"</h1>
        error_boundary(
            fallback: |error| Ok(
                    view! {
                        <p>
                            "The visit stats are unavailable: "
                            (error.to_string())
                        </p>
                    },
                ),
            suspense(fallback: view! { <p>"Counting visits..."</p> }, visits())
        )
        suspense(fallback: view! { <p>"Counting orders..."</p> }, orders())
    })
}

#[component]
async fn visits() -> Result<impl View> {
    let visits = fetch_visits().await?;
    Ok(view! {
        <p>
            (visits)
            " visits today"
        </p>
    })
}

// Stands in for an analytics service that is down.
async fn fetch_visits() -> Result<u32> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Err(std::io::Error::other("the analytics service is unreachable").into())
}

#[component]
async fn orders() -> Result<impl View> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(view! { <p>"3 orders today"</p> })
}

// The session check finishes after the page committed with the fallback, so
// its redirect can no longer become a redirect response. It streams to the
// browser as a client-side navigation to the login page instead.
#[page("/profile")]
async fn profile() -> Result<impl View> {
    Ok(view! {
        <h1>"Profile"</h1>
        suspense(
            fallback: view! { <p>"Checking your session..."</p> },
            profile_details()
        )
    })
}

#[component]
async fn profile_details(cx: &Cx) -> Result<impl View> {
    let name = fetch_session_user(cx).await?;
    Ok(view! {
        <p>
            "Signed in as "
            (name)
        </p>
    })
}

// Stands in for a slow session lookup that comes back empty.
async fn fetch_session_user(cx: &Cx) -> Result<&'static str> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    Err(redirect(href!(login).resolve(cx)).into())
}

// Where the profile page lands. A redirect thrown before the response
// commits, like one returned straight from a page handler, would arrive as a
// real HTTP redirect instead.
#[page("/login")]
async fn login() -> Result<impl View> {
    Ok(view! {
        <h1>"Log in"</h1>
        <p>
            "The profile page redirected here after it had already started streaming."
        </p>
    })
}
