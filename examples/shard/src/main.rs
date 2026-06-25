use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, procedure},
    view::view,
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
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
                topcoat::runtime::script()
            </head>
            <body>
                signal input = String::new();

                <input
                    :value=$(input.get())
                    @change=$(|e: Event| input.set(e.target.value))
                >

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
    }
}

#[procedure]
pub async fn print_on_server(input: String) -> Result<String> {
    println!("{input}");
    Ok(format!("message received: {input}"))
}
