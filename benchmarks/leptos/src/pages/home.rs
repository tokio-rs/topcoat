use leptos::prelude::*;

use crate::{components::ProductCard, server_fns::get_home};

#[component]
pub fn HomePage() -> impl IntoView {
    let data = Resource::new(|| (), |()| get_home());

    view! {
        <Suspense fallback=|| ()>
            {move || Suspend::new(async move {
                data.await.ok().map(|data| {
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
                                {data
                                    .featured
                                    .into_iter()
                                    .map(|product| view! { <ProductCard product/> })
                                    .collect_view()}
                            </div>
                        </section>
                    }
                })
            })}
        </Suspense>
    }
}
