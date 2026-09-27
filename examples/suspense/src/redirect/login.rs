use topcoat::{Result, router::page, view::{View, view}};

// Where the redirect lands. A redirect thrown before the response commits,
// like one returned straight from a page handler, would arrive as a real HTTP
// redirect instead.
#[page]
pub async fn login() -> Result<impl View> {
    Ok(view! {
        <h1>"Log in"</h1>
        <p>"The page redirected here after it had already started streaming."</p>
    })
}
