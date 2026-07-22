mod install;

use topcoat::{
    Result,
    router::{layout, page},
    view::view,
};

// A layout in app::docs wraps /docs and child routes such as /docs/install.
#[layout]
async fn docs_layout(slot: Result) -> Result {
    view! {
        <section>
            <p>"docs layout"</p>
            (slot?)
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
