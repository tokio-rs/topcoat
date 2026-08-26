use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

// Child modules append their segment: docs/install.rs -> /docs/install.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"install"</h1>
        <p>"src/app/docs/install.rs -> /docs/install"</p>
    })
}
