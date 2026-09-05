use std::time::Duration;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, shard, signal},
    view::{View, component, view},
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

                // Signals and shards need the browser runtime.
                topcoat::runtime::script()
            </head>

            <body>combobox()</body>
        </html>
    })
}

#[component]
async fn combobox(cx: &Cx) -> Result<impl View> {
    let input = signal(cx, String::new);

    Ok(view! {
        <div>
            <input :value=$(input.get()) @input=$(|e: Event| input.set(e.target.value))>

            // The shard renders again on the server whenever `input` changes.
            combobox_content(input: $(input.get()))
        </div>
    })
}

#[shard]
async fn combobox_content(cx: &Cx, input: String) -> Result<impl View> {
    // The input comes from the client, so a real application would validate it.
    let results = search_fruit(cx, &input).await;

    Ok(view! {
        <div>
            <b>"results:"</b>

            for item in results {
                <div>(item)</div>
            }
        </div>
    })
}

// Simulate a server-side lookup that takes half a second.
async fn search_fruit(_cx: &Cx, input: &str) -> Vec<&'static str> {
    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;

    let needle = input.to_lowercase();

    FRUIT
        .into_iter()
        .filter(|fruit| fruit.contains(&needle))
        .collect()
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
