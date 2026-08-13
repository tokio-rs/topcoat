mod install;

use topcoat::{
    Result,
    router::{layout, page},
    view::{ViewHandle, view},
};

// A layout in app::docs wraps /docs and child routes such as /docs/install.
#[layout]
async fn docs_layout(slot: ViewHandle<'_>) -> Result {
    view! {
        <section>
            <p>"docs layout"</p>
            (slot)
        </section>
    }
}

// A page in app::docs renders at /docs.
#[page]
async fn docs_index() -> Result {
    view! {
        <h1>"docs"</h1>
        <p>"src/app/docs.rs -> /docs"</p>
    }
}
