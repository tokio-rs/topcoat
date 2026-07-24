use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, page},
    runtime::{Signal, SignalDeclaration},
    view::view,
};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .page(order)
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

// An order form whose line items come from data, not source code: one signal
// per item is created with `Signal::new`, announced to the browser with
// `SignalDeclaration`, and captured by runtime expressions through view-level
// `let` bindings. Every interaction below runs entirely in the browser.
#[page("/")]
async fn order() -> Result {
    // In a real application these rows would come from a database.
    let items: Vec<(String, f64)> = vec![
        ("pencil".to_string(), 2.0),
        ("notebook".to_string(), 4.0),
        ("backpack".to_string(), 1.0),
    ];

    // One signal per row, created at run time.
    let quantities: Vec<Signal<f64>> = items.iter().map(|(_, q)| Signal::new(*q)).collect();
    let prices: Vec<f64> = vec![0.5, 3.0, 24.0];

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Dynamic signals"</title>
                topcoat::runtime::script()
            </head>
            <body>
                <h1>"Order"</h1>

                // Announce each runtime-created signal to the browser.
                for signal in &quantities {
                    (SignalDeclaration::new(signal))
                }

                for (index, (name, _)) in items.iter().enumerate() {
                    // A view-level `let` gives the expression a name to capture.
                    let quantity = &quantities[index];
                    let price = prices[index];
                    <div>
                        <button @click=$(|_e| quantity.set(quantity.get() - 1.0))>"-"</button>
                        " " $(quantity.get()) " "
                        <button @click=$(|_e| quantity.set(quantity.get() + 1.0))>"+"</button>
                        " " (name) " at $" (price)
                    </div>
                }

                // An expression may combine any of the runtime-created signals.
                let pencils = &quantities[0];
                let notebooks = &quantities[1];
                let backpacks = &quantities[2];
                <p>
                    <b>"Total: $"</b>
                    $(pencils.get() * 0.5 + notebooks.get() * 3.0 + backpacks.get() * 24.0)
                </p>
            </body>
        </html>
    }
}
