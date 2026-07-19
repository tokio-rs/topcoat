mod _components;
mod products;

use topcoat::{
    Result,
    asset::{Asset, AssetBundle, RouterBuilderAssetExt, asset},
    context::{Cx, app_context},
    router::{Compression, Router, Slot, layout, page},
    view::view,
};

use crate::{
    app::_components::{product_card, site_footer, site_nav},
    catalog::Catalog,
};

const TAILWIND_CSS: Asset = asset!("assets/tailwind.css");

pub fn router() -> Router {
    topcoat::router::module_router!()
        .app_context(Catalog::load())
        .assets(AssetBundle::load().expect("asset bundle loads; run `topcoat asset bundle`"))
        // Off in every benchmarked framework: the benchmark measures the
        // frameworks, not a compression codec (see the fairness notes).
        .compression(Compression::off())
        .build()
}

#[layout]
async fn root_layout(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Meridian Supply"</title>
                <link rel="stylesheet" href=(TAILWIND_CSS)>
            </head>
            <body class="flex min-h-screen flex-col bg-slate-50 text-slate-900">
                site_nav()
                <main class="mx-auto w-full max-w-6xl flex-1 px-4 py-8">
                    (slot.await?)
                </main>
                site_footer()
            </body>
        </html>
    }
}

#[page]
async fn home(cx: &Cx) -> Result {
    view! {
        <section class="rounded-2xl bg-indigo-600 px-8 py-16 text-white">
            <h1 class="max-w-2xl text-4xl font-bold tracking-tight">
                "Gear that earns its place on your desk"
            </h1>
            <p class="mt-4 max-w-xl text-lg text-indigo-100">
                "Five hundred products, zero filler. Everything in the catalog is tested daily by the people who build it."
            </p>
            <a
                href="/products"
                class="mt-8 inline-block rounded-lg bg-white px-6 py-3 text-sm font-semibold text-indigo-700 hover:bg-indigo-50"
            >
                "Browse all products"
            </a>
        </section>
        <section class="mt-12">
            <div class="flex items-baseline justify-between">
                <h2 class="text-2xl font-bold tracking-tight">"Featured products"</h2>
                <a
                    href="/products"
                    class="text-sm font-medium text-indigo-600 hover:text-indigo-500"
                >
                    "View all"
                </a>
            </div>
            <div class="mt-6 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4">
                for product in app_context::<Catalog>(cx).featured() {
                    product_card(product: product)
                }
            </div>
        </section>
    }
}
