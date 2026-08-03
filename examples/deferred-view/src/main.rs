use std::time::Duration;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{component, defer_script, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::builder()
            .discover()
            .assets(AssetBundle::load().unwrap())
            .build(),
    )
    .await
    .unwrap();
}

#[page]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Deferred views"</title>
                defer_script()
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Dashboard"</h1>
                dashboard()
            </body>
        </html>
    }
}

#[component]
async fn dashboard() -> Result {
    let activity =
        view! { <p aria-busy="true">"Loading recent activity..."</p> }?.defer(|cx| async move {
            let cx = cx.as_ref();
            view! { cx => recent_activity() }
        });

    view! {
        <main>
            <section>
                <h2>"Recent activity"</h2>
                (activity)
            </section>
            recommendations_panel()
        </main>
    }
}

#[component]
async fn recommendations_panel() -> Result {
    view! {
        <section>
            <h2>"Recommendations"</h2>
            defer recommendations() {
                <p aria-busy="true">"Loading recommendations..."</p>
            }
        </section>
    }
}

#[component]
async fn recent_activity() -> Result {
    tokio::time::sleep(Duration::from_secs(1)).await;
    view! { <p>"You published a new post."</p> }
}

#[component]
async fn recommendations() -> Result {
    tokio::time::sleep(Duration::from_secs(2)).await;
    view! { <p>"Follow the Topcoat release feed."</p> }
}
