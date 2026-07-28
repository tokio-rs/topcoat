use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, procedure},
    view::view,
};

#[tokio::main]
async fn main() {
    // Load the browser runtime assets, discover the page and procedure,
    // and start the server at http://127.0.0.1:3000 by default.
    topcoat::start(
        Router::builder()
            .assets(AssetBundle::load().unwrap())
            .discover()
            .build(),
    )
    .await
    .unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                // Reload the browser automatically during development.
                topcoat::dev::script()

                // Load the Topcoat browser runtime required by signals,
                // event handlers, and procedure calls.
                topcoat::runtime::script()
            </head>
            <body>
                // This signal stores the current value of the input in the browser.
                signal input = String::new();

                <input
                    :value=$(input.get())
                    @change=$(|e: Event| input.set(e.target.value))
                >
                // Keep the input value synchronized with the signal.

                // Update the signal when the input change event fires.

                <button
                    @click=$(async |_e| {
                        let server_response = print_on_server(input.get()).await;
                        input.set(server_response);
                    })
                >
                    // Call the Rust procedure on the server and replace the input
                    // value with the response returned by the server.
                    "Print on server"
                </button>
            </body>
        </html>
    }
}

// A procedure is an async Rust function that can be called from a browser
// runtime expression. Its arguments must be treated as untrusted input.
#[procedure]
pub async fn print_on_server(input: String) -> Result<String> {
    println!("{input}");
    Ok(format!("message received: {input}"))
}
