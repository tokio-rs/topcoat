use std::time::Duration;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, page},
    runtime::{Event, RouterBuilderRuntimeExt, shard, signal},
    view::{View, component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .assets(AssetBundle::load().unwrap())
            .runtime()
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

            <body>search()</body>
        </html>
    })
}

#[component]
async fn search(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, String::new);

    Ok(view! {
        <div>
            <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

            // The shard renders again on the server whenever `query` changes.
            search_results(query: $(query.get()))
        </div>
    })
}

#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl View> {
    // State the shard keeps for itself. Its current value travels with every
    // re-render request, so it survives the re-renders `query` triggers
    // instead of starting over at five.
    let limit = signal(cx, || 5.0);

    // The query and the limit come from the client, so a real application
    // would validate them. Reading the limit on the server makes the shard
    // depend on it: the button below changes it in the browser, and only the
    // shard renders again, not the page around it.
    let results = search_fruit(cx, &query).await;
    let shown = limit.get() as usize;

    Ok(view! {
        <div>
            <b>"results:"</b>

            for item in results.iter().take(shown) {
                <div>(item)</div>
            }

            if results.len() > shown {
                <button @click=$(|_e| limit.set(limit.get() + 5.0))>"show more"</button>
            }
        </div>
    })
}

// Simulate a server-side lookup that takes half a second.
async fn search_fruit(_cx: &Cx, query: &str) -> Vec<&'static str> {
    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;

    let needle = query.to_lowercase();

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
