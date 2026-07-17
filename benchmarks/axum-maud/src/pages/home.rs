use std::sync::Arc;

use axum::extract::State;
use maud::{Markup, html};

use crate::{
    catalog::Catalog,
    components::{layout, product_card},
};

pub async fn home(State(catalog): State<Arc<Catalog>>) -> Markup {
    layout(html! {
        section class="rounded-2xl bg-indigo-600 px-8 py-16 text-white" {
            h1 class="max-w-2xl text-4xl font-bold tracking-tight" {
                "Gear that earns its place on your desk"
            }
            p class="mt-4 max-w-xl text-lg text-indigo-100" {
                "Five hundred products, zero filler. Everything in the catalog is tested daily by the people who build it."
            }
            a href="/products"
                class="mt-8 inline-block rounded-lg bg-white px-6 py-3 text-sm font-semibold text-indigo-700 hover:bg-indigo-50" {
                "Browse all products"
            }
        }
        section class="mt-12" {
            div class="flex items-baseline justify-between" {
                h2 class="text-2xl font-bold tracking-tight" { "Featured products" }
                a href="/products" class="text-sm font-medium text-indigo-600 hover:text-indigo-500" {
                    "View all"
                }
            }
            div class="mt-6 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4" {
                @for product in catalog.featured() {
                    (product_card(product))
                }
            }
        }
    })
}
