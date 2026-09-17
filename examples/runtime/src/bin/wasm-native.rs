//! Native Rust expressions extracted from real Topcoat render bodies.

#[cfg(topcoat_wasm)]
mod app {
    use topcoat::{
        Result,
        client::Text,
        context::Cx,
        router::page,
        runtime::{shard, signal},
        view::{View, ViewExt, component, view},
    };

    pub struct Product {
        pub description: Text,
        pub price: f64,
        pub quantity: u32,
        pub extras: Vec<f64>,
    }

    impl Product {
        pub async fn load() -> Self {
            let _server_only = std::process::id();
            tokio::task::yield_now().await;
            Self {
                description: "Coffee".into(),
                price: 10.0,
                quantity: 2,
                extras: vec![1.0, 2.0],
            }
        }
    }

    fn discounted(price: f64) -> f64 {
        price * 0.9
    }

    #[component]
    async fn card(cx: &Cx, product: Product) -> Result<impl View> {
        let description = signal(cx, || product.description);
        let price = signal(cx, || product.price);
        let quantity = signal(cx, || product.quantity);
        let visible = signal(cx, || true);
        let extras = product.extras.iter().sum::<f64>();
        let suffix = Text::from(" \u{2615}\u{1f680}e\u{301}");
        Ok(view! {
            <section class="product">
                <h2>$(description.get())</h2>
                <p class="quantity">$(quantity.get())</p>
                <p class="total">
                    $(discounted(price.get()) * f64::from(quantity.get()) + extras)
                </p>
                <p class="empty">$(description.get().is_empty())</p>
                <span class="visibility" :hidden=$(!visible.get())>"Shown"</span>
                <button class="add" @click=$(|_e| quantity.set(quantity.get() + 1))>
                    "Add"
                </button>
                <button
                    class="rename"
                    @click=$(|_e| {
                        let before = description.get();
                        let snapshot = before.clone();
                        description.set(before.concat(&suffix));
                        visible.set(before == snapshot);
                    })
                >
                    "Append text"
                </button>
                <button class="toggle" @click=$(|_e| visible.set(!visible.get()))>
                    "Toggle"
                </button>
            </section>
        })
    }

    #[shard]
    async fn summary(cx: &Cx) -> Result<impl View> {
        let price = signal(cx, || 5.0);
        let product = Product {
            description: "Tea".into(),
            price: 5.0,
            quantity: 1,
            extras: vec![],
        };
        Ok(view! {
            <aside id="summary">
                <p class="shard-price">$(price.get())</p>
                card(product: product)
            </aside>
        })
    }

    #[page]
    async fn page(cx: &Cx) -> Result<impl View> {
        let product = Product::load().await;
        let price = signal(cx, || product.price);
        let other = Product {
            description: "Cake".into(),
            price: 4.0,
            quantity: 1,
            extras: vec![],
        };
        Ok(view! {
            <h1>"Native Rust in Topcoat"</h1>
            <p id="price">$(price.get())</p>
            card(product: product)
            card(product: other)
            summary()
        })
    }

    pub async fn render() -> Result<()> {
        let cx = &Cx::default();
        let html = view! { cx => page() }.single().await?.render(cx);
        println!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>Topcoat native Wasm</title></head><body>{html}<script type=\"module\" src=\"./app.js\"></script></body></html>"
        );
        Ok(())
    }
}

#[cfg(topcoat_wasm)]
#[tokio::main]
async fn main() -> topcoat::Result<()> {
    app::render().await
}

#[cfg(not(topcoat_wasm))]
fn main() {
    eprintln!("Build with tools/topcoat-split/examples/topcoat.mjs");
}
