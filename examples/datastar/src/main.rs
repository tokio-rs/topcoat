mod increment;

use serde::{Deserialize, Serialize};
use topcoat::{Result, router::{href, module_router, page}, view::{View, view}};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

#[page]
async fn home() -> Result<impl View> {
    // Datastar keeps the counter in the browser and sends it along with every
    // action request.
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Datastar"</title>

                <script
                    type="module"
                    src="https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.2/bundles/datastar.js"
                ></script>

                topcoat::dev::script()
            </head>

            <body data-signals:count="0">
                <h1>
                    "Count: "
                    <span data-text="$count"></span>
                </h1>

                // The route's URL is interpolated into the Datastar action.
                <button
                    data-on:click=(("@post('", href!(crate::increment::increment), "')"))
                >
                    "Increment"
                </button>

                <ol id="log"></ol>
            </body>
        </html>
    })
}

// Matches the signals declared by the page.
#[derive(Deserialize, Serialize)]
struct Counter {
    count: u64,
}
