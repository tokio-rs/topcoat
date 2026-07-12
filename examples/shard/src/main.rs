use std::time::Duration;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, shard},
    view::{component, view},
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
                combobox()
            </body>
        </html>
    }
}

#[component]
async fn combobox() -> Result {
    view! {
        signal input = String::new();

        <div>
            <input
                :value=$(input.get())
                @input=$(|e: Event| input.set(e.target.value))
            >

            combobox_content(input: $(input.get()))
        </div>
    }
}

#[shard]
async fn combobox_content(cx: &Cx, input: String) -> Result {
    let results = search_fruit(cx, &input).await;
    view! {
        cx =>
        <div>
            <b>"results:"</b>
            for item in results {
                <div>(item)</div>
            }
        </div>
    }
}

// Example data lookup that can only be done on the server:
async fn search_fruit(_cx: &Cx, input: &str) -> Vec<&'static str> {
    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;
    let needle = input.to_lowercase();
    FRUIT.into_iter().filter(|s| s.contains(&needle)).collect()
}

const FRUIT: [&str; 35] = [
    "apple",
    "apricot",
    "banana",
    "blackberry",
    "blueberry",
    "cherry",
    "coconut",
    "cranberry",
    "date",
    "dragonfruit",
    "elderberry",
    "fig",
    "grape",
    "grapefruit",
    "guava",
    "honeydew",
    "kiwi",
    "lemon",
    "lime",
    "lychee",
    "mango",
    "nectarine",
    "orange",
    "papaya",
    "passionfruit",
    "peach",
    "pear",
    "persimmon",
    "pineapple",
    "plum",
    "pomegranate",
    "raspberry",
    "strawberry",
    "tangerine",
    "watermelon",
];
