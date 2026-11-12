use std::sync::Arc;

use leptos::{either::Either, prelude::*};
use serde::{Deserialize, Serialize};

use super::ErrorPage;
use crate::{
    components::{PageTitle, ProductCard, icons},
    format::Count,
    model::{Category, PageError, ProductSummary},
    urls::ListingUrl,
};

/// The data of the home page.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HomeData {
    categories: Arc<[Category]>,
    featured: Vec<ProductSummary>,
}

/// Loads the categories and the featured products.
#[server]
pub async fn load_home() -> Result<HomeData, PageError> {
    let store = expect_context::<Arc<crate::store::Store>>();
    Ok(HomeData {
        categories: store.categories().clone(),
        featured: store
            .featured()
            .map(|product| store.summary(product))
            .collect(),
    })
}

/// `GET /`: hero, store perks, category links and featured products.
#[component]
pub fn HomePage() -> impl IntoView {
    let data = Resource::new(|| (), |()| load_home());
    view! {
        <Suspense>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(data) => Either::Left(view! { <Home data /> }),
                    Err(error) => Either::Right(view! { <ErrorPage error /> }),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Home(data: HomeData) -> impl IntoView {
    let perks = [
        (
            icons::truck().into_any(),
            "Free shipping over $75",
            "Orders ship within one business day from Portland.",
        ),
        (
            icons::refresh().into_any(),
            "30-day returns",
            "Changed your mind? Send it back, on us.",
        ),
        (
            icons::shield().into_any(),
            "Repair, don't replace",
            "Spare parts and repairs for as long as we sell it.",
        ),
    ];
    view! {
        <PageTitle />
        <section class="relative overflow-hidden bg-teal-900 text-white">
            <div class="mx-auto grid max-w-7xl items-center gap-12 px-4 py-20 sm:px-6 lg:grid-cols-2 lg:px-8 lg:py-28">
                <div>
                    <p class="text-sm font-semibold uppercase tracking-widest text-teal-200">
                        "New season, same harbor"
                    </p>
                    <h1 class="mt-4 text-4xl font-semibold tracking-tight text-balance sm:text-5xl lg:text-6xl">
                        "Well-made gear for every harbor day."
                    </h1>
                    <p class="mt-6 max-w-xl text-lg leading-8 text-teal-100">
                        "Tents, knives, boots and blankets from small workshops we know by name. Built to last, easy to repair and shipped free over $75."
                    </p>
                    <div class="mt-10 flex flex-wrap gap-4">
                        <a
                            href="/products"
                            class="rounded-full bg-white px-6 py-3 text-sm font-semibold text-teal-900 shadow-sm hover:bg-teal-50"
                        >
                            "Shop all products"
                        </a>
                        <a
                            href=ListingUrl::category("outdoor").to_string()
                            class="rounded-full px-6 py-3 text-sm font-semibold text-white ring-1 ring-inset ring-white/40 hover:bg-white/10"
                        >
                            "Browse outdoor gear"
                        </a>
                    </div>
                </div>
                <div aria-hidden="true" class="relative hidden aspect-[4/3] lg:block">
                    <div class="absolute left-[8%] top-[10%] aspect-square w-[46%] -rotate-6 rounded-[28%] bg-amber-200 shadow-2xl shadow-black/30"></div>
                    <div class="absolute right-[6%] top-[4%] aspect-square w-[34%] rotate-12 rounded-full bg-sky-400 shadow-2xl shadow-black/30"></div>
                    <div class="absolute bottom-[2%] right-[18%] aspect-square w-[40%] rotate-6 rounded-[28%] bg-orange-700 shadow-2xl shadow-black/30"></div>
                </div>
            </div>
        </section>
        <section aria-label="Why Harbor Market" class="border-b border-stone-200 bg-white">
            <ul class="mx-auto grid max-w-7xl gap-8 px-4 py-10 sm:grid-cols-3 sm:px-6 lg:px-8">
                {perks
                    .map(|(icon, title, text)| {
                        view! {
                            <li class="flex gap-4">
                                <span class="flex size-11 shrink-0 items-center justify-center rounded-full bg-teal-50 text-teal-700">
                                    {icon}
                                </span>
                                <div>
                                    <p class="font-semibold text-stone-900">{title}</p>
                                    <p class="mt-1 text-sm text-stone-600">{text}</p>
                                </div>
                            </li>
                        }
                    })}
            </ul>
        </section>
        <section
            aria-labelledby="categories-heading"
            class="mx-auto max-w-7xl px-4 pt-16 sm:px-6 lg:px-8"
        >
            <h2 id="categories-heading" class="text-2xl font-semibold tracking-tight">
                "Shop by category"
            </h2>
            <ul class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {data
                    .categories
                    .iter()
                    .map(|category| {
                        view! {
                            <li>
                                <a
                                    href=ListingUrl::category(&category.slug).to_string()
                                    class="group block h-full rounded-2xl border border-stone-200 bg-white p-5 transition hover:border-teal-600 hover:shadow-md"
                                >
                                    <h3 class="font-semibold text-stone-900 group-hover:text-teal-800">
                                        {category.name.clone()}
                                    </h3>
                                    <p class="mt-1 line-clamp-2 text-sm text-stone-600">
                                        {category.description.clone()}
                                    </p>
                                    <p class="mt-4 text-xs font-medium text-stone-500">
                                        {format!("{} products", Count(category.product_count as u64))}
                                    </p>
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
        <section
            aria-labelledby="featured-heading"
            class="mx-auto max-w-7xl px-4 pt-20 sm:px-6 lg:px-8"
        >
            <div class="flex items-end justify-between gap-4">
                <h2 id="featured-heading" class="text-2xl font-semibold tracking-tight">
                    "Featured products"
                </h2>
                <a
                    href="/products"
                    class="flex items-center gap-1.5 text-sm font-semibold text-teal-700 hover:text-teal-900"
                >
                    "View all"
                    {icons::arrow_right()}
                </a>
            </div>
            <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
                {data
                    .featured
                    .into_iter()
                    .map(|product| view! { <ProductCard product /> })
                    .collect_view()}
            </div>
        </section>
    }
}
