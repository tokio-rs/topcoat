use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, Slot, layout, page},
    tailwind,
    view::view,
};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .layout(root_layout)
        .page(home)
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <html>
            <head>
                topcoat::dev::script()
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body class="p-8 font-sans">
                (slot.await?)
            </body>
        </html>
    }
}

#[page("/")]
async fn home() -> Result {
    view! {
        <main class="max-w-md rounded-lg border border-slate-200 bg-slate-50 p-6">
            <h1 class="text-2xl font-bold text-slate-900">"Tailwind works"</h1>
            <p class="mt-2 text-slate-600">"This page uses basic Tailwind utility classes."</p>
            <button class="mt-4 rounded bg-blue-600 px-4 py-2 font-semibold text-white">
                "Button"
            </button>
        </main>
    }
}
