use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, view},
};

// Child modules append their segment: docs/install.rs -> /docs/install.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"install"</h1>
        <p>"src/app/docs/install.rs -> /docs/install"</p>
    })
}
