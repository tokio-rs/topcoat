use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    router::{error::redirect, href, page},
    view::{View, component, suspense, view},
};

// The check finishes after the page committed with the fallback, so its
// redirect can no longer become a redirect response. It streams to the
// browser as a client-side navigation to the login page instead.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Redirect"</h1>
        suspense(fallback: view! { <p>"Checking your session..."</p> }, content())
    })
}

#[component]
async fn content(cx: &Cx) -> Result<impl View> {
    let name = load_session_user(cx).await?;
    Ok(view! {
        <p>
            "Signed in as "
            (name)
        </p>
    })
}

// Stands in for a slow session lookup that comes back empty.
async fn load_session_user(cx: &Cx) -> Result<&'static str> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    Err(redirect(href!(login).resolve(cx)).into())
}

// Where the redirect lands. A redirect thrown before the response commits,
// like one returned straight from a page handler, would arrive as a real HTTP
// redirect instead.
#[page("./login")]
pub async fn login() -> Result<impl View> {
    Ok(view! {
        <h1>"Log in"</h1>
        <p>"The page redirected here after it had already started streaming."</p>
    })
}
