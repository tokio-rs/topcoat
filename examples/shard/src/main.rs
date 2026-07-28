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
    // Load the browser runtime assets, discover the page and shard endpoint,
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
                // Enable automatic browser reload during development.
                topcoat::dev::script()

                // Load the Topcoat browser runtime used by signals and shards.
                topcoat::runtime::script()
            </head>

            <body>combobox()</body>
        </html>
    }
}

#[component]
async fn combobox() -> Result {
    view! {
        // Store the current search text as reactive browser state.
        signal input = String::new();

        <div>
            // Update the signal whenever the user types.
            <input :value=$(input.get()) @input=$(|e: Event| input.set(e.target.value))>

            // Re-render this shard on the server whenever `input` changes.
            combobox_content(input: $(input.get()))
        </div>
    }
}

#[shard]
async fn combobox_content(cx: &Cx, input: String) -> Result {
    // Run the search on the server and render the matching results.
    let results = search_fruit(cx, &input).await;

    view! {
        <div>
            <b>"results:"</b>

            for item in results {
                <div>(item)</div>
            }
        </div>
    }
}

// Simulate a server-side lookup that takes half a second.
async fn search_fruit(_cx: &Cx, input: &str) -> Vec<&'static str> {
    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;

    // Perform a case-insensitive substring search.
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
