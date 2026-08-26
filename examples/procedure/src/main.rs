use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, procedure},
    view::{View, view},
};

#[tokio::main]
async fn main() {
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
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()

                // Signals, event handlers, and procedure calls need the
                // browser runtime.
                topcoat::runtime::script()
            </head>
            <body>
                signal input = String::new();

                // `:value` renders the signal, `@change` writes back to it.
                <input
                    :value=$(input.get())
                    @change=$(|e: Event| input.set(e.target.value))
                >

                // Calls the procedure on the server and puts its return value
                // back into the signal.
                <button
                    @click=$(async |_e| {
                        let server_response = print_on_server(input.get()).await;
                        input.set(server_response);
                    })
                >
                    "Print on server"
                </button>
            </body>
        </html>
    })
}

// The arguments come from the client, so a real application would validate
// them here.
#[procedure]
pub async fn print_on_server(input: String) -> Result<String> {
    println!("{input}");
    Ok(format!("message received: {input}"))
}
