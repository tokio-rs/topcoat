use topcoat::{
    Result,
    asset::AssetBundle,
    context::Cx,
    router::{Router, action, page},
    runtime::Event,
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::new()
            .assets(AssetBundle::load().unwrap())
            .discover(),
    )
    .await
    .unwrap();
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
                topcoat::runtime::script()
            </head>
            <body>
                signal input = "".to_owned();

                <input
                    :value=$(input.get())
                    @change=$(|e: Event| input.set(e.target.value))
                >

                <button
                    @click=$(async |_e| {
                        let server_response = print_on_server(input.get()).await;
                        input.set("".to_owned());
                        raw!("console.log(${server_response})");
                    })
                >
                    "Print on server"
                </button>
            </body>
        </html>
    }
}

#[action]
pub async fn print_on_server(input: String) -> Result<String> {
    println!("{}", input);
    Ok("Message received!".to_owned())
}
