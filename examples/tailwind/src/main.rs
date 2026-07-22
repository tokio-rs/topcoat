use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, layout, page},
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
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::dev::script()
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body
                class="flex min-h-screen items-center justify-center bg-slate-100 font-sans"
            >
                (slot?)
            </body>
        </html>
    }
}

#[page("/")]
async fn home() -> Result {
    view! {
        <main
            class="mx-4 w-full max-w-md rounded-2xl bg-white p-8 shadow-lg ring-1 ring-slate-200"
        >
            // Tailwind's `hidden` utility removes this warning, so it only
            // appears when the stylesheet is missing.
            <p class="hidden">"Tailwind is not working: this page should look styled."
            </p>
            // The opposite: the inline style hides this badge until Tailwind's
            // important `flex!` utility overrides it.
            <p
                style="display: none"
                class="flex! w-fit items-center gap-2 rounded-full bg-emerald-100 px-3 py-1 text-sm font-medium text-emerald-800"
            >
                <span class="size-2 rounded-full bg-emerald-500"></span>
                "Tailwind is working"
            </p>
            <h1 class="mt-4 text-2xl font-bold tracking-tight text-slate-900">
                "Topcoat + Tailwind"
            </h1>
            <p class="mt-2 text-slate-600">
                "Utility classes in your Rust sources are compiled into this page's stylesheet by the standalone Tailwind CLI."
            </p>
            <a
                href="https://tailwindcss.com/docs"
                class="mt-6 inline-block rounded-lg bg-blue-600 px-4 py-2 font-semibold text-white shadow-sm hover:bg-blue-500"
            >
                "Read the Tailwind docs"
            </a>
        </main>
    }
}
