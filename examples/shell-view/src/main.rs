use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    router::{Router, RouterBuilderDiscoverExt, route},
    shell_view::ShellView,
    view::{component, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[route(GET "/")]
async fn home(cx: &Cx) -> Result<ShellView> {
    let mut page = ShellView::builder(cx);
    let activity = page.defer(
        view! { <p aria-busy="true">"Loading recent activity..."</p> }?,
        |cx| async move {
            let cx = cx.as_ref();
            view! { cx => recent_activity() }
        },
    );
    let recommendations_slot = page.defer(
        view! { <p aria-busy="true">"Loading recommendations..."</p> }?,
        |cx| async move {
            let cx = cx.as_ref();
            view! { cx => recommendations() }
        },
    );

    let shell = view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Shell view"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Dashboard"</h1>
                <section>
                    <h2>"Recent activity"</h2>
                    (activity)
                </section>
                <section>
                    <h2>"Recommendations"</h2>
                    (recommendations_slot)
                </section>
            </body>
        </html>
    }?;
    Ok(page.finish(shell))
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
