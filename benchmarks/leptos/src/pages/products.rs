use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::{
    components::{PaginationNav, ProductCard},
    format::{normalize_sort, products_url},
    model::ProductsData,
    server_fns::get_products,
};

const SORT_OPTIONS: [(Option<&str>, &str); 5] = [
    (None, "Default"),
    (Some("name"), "Name"),
    (Some("price"), "Price: Low to high"),
    (Some("price-desc"), "Price: High to low"),
    (Some("rating"), "Rating"),
];

const CHIP_ACTIVE: &str = "rounded-full bg-slate-900 px-3 py-1 font-medium text-white";
const CHIP_INACTIVE: &str =
    "rounded-full bg-white px-3 py-1 font-medium text-slate-600 shadow-sm hover:bg-slate-100";

#[component]
pub fn ProductsPage() -> impl IntoView {
    let query = use_query_map();
    let data = Resource::new(
        move || {
            let query = query.get();
            let page = query
                .get("page")
                .and_then(|page| page.parse::<usize>().ok())
                .unwrap_or(1);
            let sort = normalize_sort(query.get("sort").as_deref()).map(str::to_owned);
            let category = query.get("category");
            (page, sort, category)
        },
        |(page, sort, category)| get_products(page, sort, category),
    );

    view! {
        <Suspense fallback=|| ()>
            {move || Suspend::new(async move { data.await.ok().map(products_view) })}
        </Suspense>
    }
}

fn products_view(data: ProductsData) -> impl IntoView {
    let ProductsData {
        categories,
        items,
        current,
        page_count,
        total,
        sort,
        category,
    } = data;

    view! {
        <div class="flex flex-wrap items-baseline justify-between gap-4">
            <h1 class="text-3xl font-bold tracking-tight">"All products"</h1>
            <p class="text-sm text-slate-500">{format!("{total} products")}</p>
        </div>
        <div class="mt-6 flex flex-wrap items-center gap-2 text-sm">
            <span class="font-medium text-slate-500">"Sort:"</span>
            {SORT_OPTIONS
                .iter()
                .map(|(value, label)| {
                    view! {
                        <a
                            href=products_url(1, *value, category.as_deref())
                            class=if *value == sort.as_deref() { CHIP_ACTIVE } else { CHIP_INACTIVE }
                        >
                            {*label}
                        </a>
                    }
                })
                .collect_view()}
        </div>
        <div class="mt-3 flex flex-wrap items-center gap-2 text-sm">
            <span class="font-medium text-slate-500">"Category:"</span>
            <a
                href=products_url(1, sort.as_deref(), None)
                class=if category.is_none() { CHIP_ACTIVE } else { CHIP_INACTIVE }
            >
                "All"
            </a>
            {categories
                .iter()
                .map(|entry| {
                    view! {
                        <a
                            href=products_url(1, sort.as_deref(), Some(&entry.slug))
                            class=if category.as_deref() == Some(entry.slug.as_str()) {
                                CHIP_ACTIVE
                            } else {
                                CHIP_INACTIVE
                            }
                        >
                            {entry.name.clone()}
                        </a>
                    }
                })
                .collect_view()}
        </div>
        {if total == 0 {
            view! { <p class="mt-8 text-slate-500">"No products found."</p> }.into_any()
        } else {
            view! {
                <div class="mt-8 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4">
                    {items
                        .into_iter()
                        .map(|product| view! { <ProductCard product/> })
                        .collect_view()}
                </div>
                <PaginationNav current page_count sort category/>
            }
                .into_any()
        }}
    }
}
