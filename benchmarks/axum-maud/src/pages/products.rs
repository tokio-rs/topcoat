use std::sync::Arc;

use axum::extract::{Query, State};
use maud::{Markup, html};
use serde::Deserialize;

use crate::{
    catalog::Catalog,
    components::{layout, pagination, product_card},
    urls::{normalize_sort, products_url},
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

#[derive(Deserialize)]
pub struct ProductsQuery {
    page: Option<usize>,
    sort: Option<String>,
    category: Option<String>,
}

pub async fn products(
    State(catalog): State<Arc<Catalog>>,
    Query(query): Query<ProductsQuery>,
) -> Markup {
    let sort = normalize_sort(query.sort.as_deref());
    let category = query.category.as_deref();
    let page = catalog.page(query.page.unwrap_or(1), sort, category);

    layout(html! {
        div class="flex flex-wrap items-baseline justify-between gap-4" {
            h1 class="text-3xl font-bold tracking-tight" { "All products" }
            p class="text-sm text-slate-500" { (page.total) " products" }
        }
        div class="mt-6 flex flex-wrap items-center gap-2 text-sm" {
            span class="font-medium text-slate-500" { "Sort:" }
            @for (value, label) in SORT_OPTIONS {
                a href=(products_url(1, value, category))
                    class=(if value == sort { CHIP_ACTIVE } else { CHIP_INACTIVE }) {
                    (label)
                }
            }
        }
        div class="mt-3 flex flex-wrap items-center gap-2 text-sm" {
            span class="font-medium text-slate-500" { "Category:" }
            a href=(products_url(1, sort, None))
                class=(if category.is_none() { CHIP_ACTIVE } else { CHIP_INACTIVE }) {
                "All"
            }
            @for entry in catalog.categories() {
                a href=(products_url(1, sort, Some(&entry.slug)))
                    class=(if category == Some(entry.slug.as_str()) { CHIP_ACTIVE } else { CHIP_INACTIVE }) {
                    (entry.name)
                }
            }
        }
        @if page.total == 0 {
            p class="mt-8 text-slate-500" { "No products found." }
        } @else {
            div class="mt-8 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4" {
                @for product in &page.items {
                    (product_card(product))
                }
            }
            (pagination(page.current, page.page_count, sort, category))
        }
    })
}
